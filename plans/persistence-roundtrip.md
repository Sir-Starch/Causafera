# Deterministic Persistence Roundtrip

**Status:** Active — Stage 1 in progress

## Goal

Complete `TODO-PERSIST-001` by defining and implementing the first bounded, versioned persistence format that can:

```text
running deterministic world at a completed tick
→ canonical snapshot export
→ bounded binary encoding
→ atomic durable write
→ validated decode and runtime reconstruction
→ continued execution
```

An uninterrupted run and a save/reload/resume run must produce identical physical-state and causal-history digests at every matched checkpoint after restoration.

This plan establishes exact snapshots and manual save/resume. It does not implement continuous autosave, distributed storage, provenance compaction, cloud synchronization, or observer streaming.

## Context

`causafera-persistence::Snapshot` currently stores only `version` and `world_seed`. It is disconnected from runtime state and cannot restore a simulation.

The post-Phase-24 implementation now includes:

- versioned `PhysicalStateDigest`, `HistoryDigest`, and `ExperimentDigest`;
- scheduler time and deterministic runtime configuration;
- complete `CausalTraceStore` event/effect/cause ancestry;
- bounded cross-tick `PhysicalPatternHistory`;
- chart-qualified multi-chunk mana fields and causal resolution;
- mana-effect hysteresis and physical feedback state;
- active chunk bookkeeping and promotion/demotion state;
- objective biological actors, subjective scenes/active cognition, and action state;
- population aggregates and concrete historical bootstrap receipts;
- experiment manifests and read-only Explanation IR/analytics.

Serializing Rust object memory or derived observer data would be incorrect. Persistence needs a canonical logical-state boundary, explicit schema revisions, bounded allocation, validated reconstruction, and exact post-load digest verification.

## Relevant invariants

INV-006, INV-007, INV-011 through INV-023, INV-027 through INV-037.

The persistence-specific consequences are:

- snapshots contain authoritative state and provenance, never localized labels or observer classifications;
- load cannot bypass proposal/commit invariants by accepting malformed state;
- serialization order is canonical and independent of hash iteration, pointer identity, locale, and scheduler execution order;
- subjective actor state remains structurally separate from authoritative entity/place/body identities;
- chart/frame scope remains explicit;
- restored history remains reconstructable;
- format and performance claims require reproducible tests and benchmarks.

## Ontology domains affected

Persistence, determinism, simulation time, runtime configuration, provenance, geometry, geography, mana, causal resolution, biology, perception/cognition state, population aggregates, historical bootstrap, experiments, analytics metadata, and observer boundaries.

## Causal carriers affected

Persistence does not create new simulation causes. It preserves existing carriers exactly:

- event IDs, trace IDs, causes, effects, times, phases, and opaque schemas;
- physical field values and their last-change traces;
- temporal pattern samples and ancestry;
- effect hysteresis/feedback state;
- actor objective state, subjective bounded state, and committed action ancestry;
- aggregate quantities, lifecycle state, bootstrap stage receipts, and active resolution state.

Snapshot write/read operations are external execution events and must not appear as Ground Truth causal events unless a future physical in-world storage mechanism explicitly models them.

## Relevant documents

- `PLANS.md`
- `docs/architecture/invariants.md`
- `docs/architecture/determinism.md`
- `docs/architecture/data-oriented.md`
- `docs/architecture/provenance.md`
- `docs/architecture/observer.md`
- `docs/architecture/protocol.md`
- `docs/observer/snapshots.md`
- `docs/performance/benchmarks.md`
- `docs/simulation/long-run-experiments.md`
- `docs/rfc/RFC-TRACE-001.md`
- `docs/rfc/RFC-GEO-002.md`
- `docs/rfc/RFC-MANA-001.md`
- `docs/rfc/RFC-RES-001.md`
- `docs/rfc/RFC-HIST-001.md`
- `docs/rfc/RFC-EXPLAIN-001.md`
- `docs/adr/ADR-002.md`
- `docs/adr/ADR-003.md`
- `docs/adr/ADR-004.md`
- `docs/adr/ADR-006.md`

## Current state

The persistence crate contains a two-field serde/JSON placeholder. No format magic, schema registry, size bounds, integrity check, domain sections, trace-store export, runtime reconstruction, migration policy, atomic file write, CLI command, or save/resume test exists.

Runtime already computes canonical state/history digests, but many authoritative fields are private and some runtime systems carry clocks/configuration needed for reconstruction. The scheduler stores trait objects that must be rebuilt from a registered runtime recipe rather than serialized as memory.

The deterministic RNG is stateless with respect to sequential consumption: streams are derived from seed, time, phase, system ID, and operation identity. Therefore snapshot restoration must preserve time, configuration, system registration order/revisions, and authoritative state; it must not persist an opaque PRNG memory image.

## Proposed architecture

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

Header fields:

- fixed magic bytes;
- format major/minor version;
- canonical codec revision;
- world seed and completed simulation time;
- runtime recipe/schema-registry fingerprint;
- physical digest schema/version and bytes;
- history digest schema/version and bytes;
- section count and directory offset;
- whole-payload integrity digest.

Each canonical section contains:

- opaque `SnapshotSectionSchemaId`;
- section schema major/minor revision;
- flags reserved for future compatible features;
- payload offset and length;
- decoded-size limit;
- per-section integrity digest;
- canonical payload bytes.

Sections are strictly ordered by schema ID and unique. Unknown required sections fail load. Unknown explicitly optional sections may be skipped only when the envelope declares that compatibility rule.

### Initial section registry

The first complete runtime snapshot includes separate bounded sections for:

1. deterministic runtime recipe and configuration;
2. spatial/chart and active-chunk state;
3. mana field set, last-change traces, and feedback/hysteresis state;
4. causal resolution field and promotion/demotion bookkeeping;
5. physical pattern history and carrier adapter revisions;
6. objective physical counters/material/activity state;
7. biological actor objective state;
8. actor subjective/active cognition state;
9. population aggregates and historical bootstrap records/receipts;
10. complete causal trace store;
11. experiment manifest needed to resume a laboratory run, when present.

Explanation IR, localized rendering, wall-clock measurements, observer subscriptions, UI state, derived caches, and performance telemetry are not authoritative sections. They are recomputed after load. If an experiment report needs archival, it uses a separate non-authoritative artifact format referencing snapshot digests.

### Provenance encoding

Persist the canonical forward representation of `CausalTraceStore`:

- IDs and next-ID counters;
- event times, phases, opaque kinds;
- ordered cause offsets/IDs;
- ordered effect offsets/targets/before/after fingerprints.

Derived child indexes are rebuilt deterministically and validated after decode instead of being duplicated. Load verifies monotonic IDs, parent-before-child ancestry, valid offsets, strict effect ordering, and referenced trace existence.

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

Use a deterministic content-integrity digest such as BLAKE3 for envelope/section corruption detection. It is not an authoritative simulation digest and makes no authenticity claim. Signed snapshots and adversarial remote storage remain out of scope.

Decode validates all lengths/counts before allocation, uses checked arithmetic, enforces global and per-section caps, rejects duplicate/overlapping sections and trailing authoritative bytes, and returns typed errors without panic.

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

## Primitive vs emergent review

Primitive persistence data:

- format/schema versions and opaque section IDs;
- canonical numeric/typed state;
- physical/history digests;
- committed causal graph;
- explicit runtime recipe and adapter revisions;
- integrity digests and byte lengths;
- simulation time and deterministic seed/configuration.

Not authoritative persistence data:

- English/Russian labels, narrative, UI layout, selected panels;
- observer classifications or localized Explanation rendering;
- wall-clock timestamps as simulation causes;
- semantic snapshot section names inside simulation state;
- inferred missing history or best-effort repaired state;
- platform pointer/layout representation.

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

## Implementation stages

### 1. Accept RFC-PERSIST-001 and freeze snapshot ontology

- Specify quiescent tick boundary, authoritative/non-authoritative inventory, dependency direction, format version rules, section compatibility, size caps, integrity semantics, and failure behavior.
- Audit every field contributing to `PhysicalStateDigest` and `HistoryDigest` against the section registry.
- Assign opaque section/schema IDs and runtime-system recipe IDs without English meaning in authoritative bytes.
- Define required versus optional sections for format v1.

Acceptance gate:

- every physical/history digest input has exactly one owning snapshot section or is explicitly deterministic derived state;
- no observer/Explanation rendering state appears in authoritative inventory;
- no crate dependency cycle is introduced;
- RFC and TODO scope are accepted before encoding begins.

### 2. Implement bounded canonical envelope and primitive codec

- Replace the JSON placeholder with explicit little-endian reader/writer primitives.
- Implement header, sorted section directory, per-section and whole-payload integrity, checked offsets, caps, and typed errors.
- Provide byte-slice and streaming file APIs with identical logical output.
- Add malformed-input tests: truncation, overflow, overlap, duplicate section, unknown required section, invalid flags, checksum mismatch, and oversized declaration.

Acceptance gate:

- identical logical sections produce byte-identical snapshots regardless of insertion order;
- decoder performs no allocation before validating declared bounds;
- fuzz/property tests find no panic for arbitrary bounded bytes;
- debug JSON, if retained, is explicitly non-authoritative and cannot be loaded as a production snapshot.

### 3. Add canonical provenance export/import

- Expose a validated logical snapshot view/constructor for `CausalTraceStore`.
- Encode forward SoA arrays and next-ID counters.
- Rebuild reverse child index on load.
- Validate IDs, offsets, ordering, causes, effects, and acyclicity-by-parent-order.

Acceptance gate:

- nontrivial multi-domain trace store roundtrips exactly;
- `HistoryDigest` matches before/after;
- parent/child traversal results are identical after reconstruction;
- corruption of an ID, offset, cause, or effect is rejected without partial load.

### 4. Add domain-state sections and validated reconstruction

- Export/import chart-qualified active chunks, mana field set, resolution state, pattern history, feedback state, physical counters/material state, actors, subjective bounded state, population aggregates, historical receipts, and experiment manifest.
- Use domain-owned validated constructors or snapshot DTOs; do not make fields public solely for serde convenience.
- Rebuild deterministic indexes/caches instead of persisting duplicates.
- Preserve hard capacities and subjective/Ground Truth boundaries on load.

Acceptance gate:

- each section has isolated roundtrip, bound, ordering, duplicate-ID, unknown-reference, and corruption tests;
- actor subjective sections cannot contain authoritative entity/place/body/chart/frame/trace IDs beyond explicitly external inaccessible bookkeeping;
- aggregate conservation and mana intensity totals survive roundtrip;
- `PhysicalStateDigest` matches exactly after all sections reconstruct.

### 5. Reconstruct runtime and scheduler at a completed tick

- Implement `Runtime::export_snapshot` and `Runtime::from_snapshot` or equivalent fallible APIs.
- Persist/resolve runtime recipe, system/adapter revisions, phase order, registration order, time, and parameters.
- Recreate scheduler systems/central state without serializing locks or trait objects.
- Require pending phase buffers to be empty at export.
- Recompute physical/history digests and reject snapshot if header values disagree.

Acceptance gate:

- restored snapshot at tick `K` reports the same time, metrics, physical digest, history digest, active chunks, actor count, and trace count;
- next tick executes each registered system exactly once in canonical order;
- unavailable or revision-mismatched required system/adapter schema fails before runtime exposure;
- no load path mutates the original runtime or partially publishes invalid state.

### 6. Prove uninterrupted equivalence and version behavior

- Run `N` ticks uninterrupted.
- Separately run `K` ticks, export/encode/decode/reconstruct, then run `N-K` ticks.
- Compare every post-restore checkpoint, physical/history/experiment digests, causal events, metrics, and Explanation IR.
- Repeat with mana feedback, temporal patterns, multiple chunks, promoted resolution, actors, population lifecycle, and historical bootstrap enabled.
- Add version tests: current minor, unknown optional section, missing required section, future major rejection, and explicit pure migration fixture.

Acceptance gate:

- uninterrupted and resumed runs are bit-identical in canonical results;
- two encode/decode cycles produce the same bytes;
- load/save without ticking is idempotent;
- version failures are deterministic and diagnostic;
- golden v1 fixtures are stored only after RFC acceptance and format freeze.

### 7. Add atomic file save/load and CLI laboratory workflow

- Implement bounded file APIs and atomic replacement.
- Add CLI commands for explicit save, inspect/validate, load/resume, and experiment checkpoint/resume.
- CLI output remains non-authoritative and never enters snapshot bytes.
- Add failure injection around write, flush, rename, integrity validation, and reconstruction.

Acceptance gate:

- interrupted write preserves the previous valid snapshot;
- load of corrupted/truncated file fails without starting a runtime;
- `run → save → resume` and `lab → checkpoint → resume` execute successfully from CLI;
- saving does not add a Ground Truth event or change physical/history digests;
- paths and wall-clock metadata do not affect authoritative bytes.

### 8. Benchmark, document, and complete TODO-PERSIST-001

- Benchmark encode/decode, validation, atomic write/read, peak temporary memory, file size, and pause duration for the demonstrated 192-tick/8-actor/≤3-chunk workload plus smaller/larger bounded variants.
- Record hardware, code/schema revision, section sizes, and digest verification.
- Decide from measurements whether longer experiments require streaming encode, compression, incremental snapshots, or provenance work in a separate RFC/ExecPlan.
- Update documentation, changelog, ontology matrices, unresolved assumptions, and plan status.

Acceptance gate:

- TODO-PERSIST-001 acceptance criteria are met by exact runtime roundtrip, not merely serde success;
- benchmarks are version-controlled and reproducible;
- no unmeasured scale claim is made;
- observer snapshot/delta docs explicitly remain separate from persistence;
- follow-up work is recorded without expanding this plan into full persistence infrastructure.

## Verification

- `cargo test --workspace --all-targets`;
- strict clippy and formatting;
- `git diff --check`;
- canonical insertion-order and repeated-encoding tests;
- malformed/fuzz/property decoder tests;
- per-section roundtrip and corruption tests;
- trace parent/child equivalence;
- same-process and fresh-process save/resume equivalence;
- physical/history digest equality at every matched checkpoint;
- locale-independence test with Explanation IR active;
- file failure-injection tests;
- actual CLI save/validate/resume laboratory run;
- codebase graph refresh and dependency/call-path audit.

Completion requires zero test failures and a recorded uninterrupted-versus-resumed experiment result.

## Benchmark plan

Workloads:

1. minimal one-chunk field at early tick;
2. Phase 24-style one-chunk long field history;
3. current coupled reference: 192 ticks, 8 actors, up to 3 chunks;
4. maximum demonstrated in-memory workload selected from existing experiment bounds;
5. malformed maximum-size header rejection without payload allocation.

Metrics:

- encoded bytes total and per section;
- encode, integrity, decode, validation, reconstruction, and digest-verification time;
- atomic write/read wall time;
- peak RSS and temporary bytes;
- pause duration at snapshot boundary;
- bytes per actor/chunk/trace;
- resumed ticks/second versus uninterrupted baseline.

No compression or incremental-save claim is accepted until this baseline identifies the dominant sections and crossover.

## Determinism impact

- Encoding is explicit little-endian and section order is canonical.
- Hash maps, pointer order, locale, file path, wall time, and OS directory ordering cannot affect bytes.
- Snapshot occurs only at completed tick `K`; system next-time state derives canonically from `K` and recipe revision.
- RNG streams resume from explicit seed/time/phase/system/operation keys, not serialized generator memory.
- Rebuilt caches/indexes cannot contribute independently to physical/history digests.
- Integrity digests detect corruption but do not replace simulation digests.
- Migrations are pure, versioned, deterministic transformations with golden fixtures.

## Memory impact

- Decoder enforces total file, section count, payload length, decoded count, actor, chunk, pattern, trace, cause, effect, aggregate, and document limits before allocation.
- The initial implementation may encode in memory only if the measured reference workload remains within a documented cap.
- Streaming encode/decode is required before larger experiments if temporary memory approaches authoritative-state memory.
- Derived child indexes/caches are rebuilt, avoiding duplicate persisted state.
- Snapshot creation must not clone the complete world more than once; per-section buffering and measured peak memory determine the final implementation.

## Observer impact

Persistence format is not the observer protocol. The observer cannot submit authoritative snapshots through its read-only API. After load, observer read models and subscriptions start from newly derived scoped snapshots with new transport sequence state while simulation time and authoritative digests remain restored.

Observer transport of Explanation IR is a follow-up plan after persistence completion.

## Explanation impact

Explanation IR and analytics are derived after load from restored authoritative state/provenance. Localized strings and LLM output are never persisted as authoritative sections. A non-authoritative archived experiment report may reference snapshot physical/history digests and supporting traces, but cannot repair or augment a snapshot.

Roundtrip verification includes deterministic Explanation IR equality to prove that restored provenance supports the same claims.

## Persistence impact

This plan creates persistence v1 and completes the minimal snapshot foundation. Future changes require:

- compatible minor section additions under explicit rules;
- pure registered migrations for supported older minor schemas;
- a new major version for incompatible container or authoritative semantic changes;
- separate plans for incremental snapshots, compression, compaction, branching, autosave, or distributed storage.

## Cross-domain effects

- Every authoritative domain gains an explicit logical snapshot/reconstruction boundary.
- Provenance remains the cross-domain ancestry source after restore.
- Geometry/chart identity and resolution state remain aligned.
- Mana history/effects and actor cognition/action resume without skipped or duplicated phase work.
- Population/history aggregates resume with conservation and receipt ancestry intact.
- Experiments can exceed one process lifetime without weakening deterministic comparisons.

## Risks

- Missing a digest-contributing field creates false roundtrip confidence.
- Serializing private Rust layout couples format to compiler implementation.
- Persisting derived caches creates duplicate authorities and stale state.
- Runtime/persistence dependency cycles encourage poor layering.
- Loading through unchecked constructors bypasses domain invariants.
- Huge declared lengths enable memory exhaustion before validation.
- Trait-object/system revisions may be unavailable at load time.
- Rebuilding system clocks incorrectly may skip or duplicate one phase.
- Provenance child indexes may diverge from forward edges.
- Best-effort migration may silently change history.
- Atomic rename/fsync behavior differs across platforms.
- Integrity hashes may be mistaken for authenticity or simulation identity.
- Snapshot pause/memory overhead may dominate longer experiments.

Mitigations are explicit logical codecs, section ownership audit, quiescent boundaries, checked caps, validated constructors, runtime recipes, digest recomputation, golden fixtures, failure injection, and benchmark gates.

## Documentation changes

On activation/completion:

- create and accept `docs/rfc/RFC-PERSIST-001.md`;
- add persistence architecture/format documentation and index entries;
- update determinism and provenance documents;
- clarify observer snapshots versus simulation persistence;
- update domain coverage, causal carriers, primitive/emergent, and unresolved assumptions;
- update TODO-PERSIST-001, changelog, rebaseline report, roadmap wording if needed, and `PLANS.md`;
- store benchmark methodology/results without scale extrapolation.

## TODO changes

On activation:

- refine `TODO-PERSIST-001` acceptance to require canonical runtime export/import, physical/history digest equality, and uninterrupted/resumed equivalence;
- depend on completed digest separation, runtime coupling, and provenance foundation;
- do not mark complete after envelope serialization alone.

Potential follow-ups, created only if measurements justify them:

- incremental/delta snapshots;
- compression;
- provenance compaction/archival;
- autosave/crash recovery;
- snapshot signing/encryption;
- observer Explanation IR transport.

## Decision log

- 2026-07-13: Snapshot v1 is taken only at a completed scheduler tick.
- 2026-07-13: Persistence uses a sectioned canonical logical binary format, not Rust memory layout, JSON, or observer protobufs.
- 2026-07-13: Persistence owns envelope/codec/I/O; runtime owns domain section assembly and scheduler reconstruction.
- 2026-07-13: Physical and history digests are stored and recomputed independently after load.
- 2026-07-13: Forward provenance state is persisted; reverse indexes are deterministically rebuilt.
- 2026-07-13: Locks, trait objects, caches, observer state, Explanation prose, and LLM output are never authoritative snapshot data.
- 2026-07-13: Initial file writes use atomic replacement and preserve the prior valid snapshot on failure.
- 2026-07-13: Unsupported major versions fail closed; migration is explicit and pure.

## Progress

- [x] RFC-PERSIST-001 accepted and snapshot ontology frozen.
- [x] Canonical bounded envelope and primitive codec implemented.
- [x] Causal provenance export/import implemented and verified.
- [x] Domain-state sections and validated reconstruction implemented.
- [x] Runtime/scheduler reconstruction at completed tick implemented.
- [x] Uninterrupted versus save/reload/resume equivalence proven.
- [x] Atomic file and CLI save/validate/resume workflow implemented.
- [x] Benchmarks/docs complete and TODO-PERSIST-001 closed.
