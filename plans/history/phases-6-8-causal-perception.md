# Phases 6–8 Causal Perception Foundation

> **Historical record.** This completed ExecPlan describes a Foundation Era project stage. Its implementation status and terminology may be outdated; use [the documentation index](../../docs/index.md), [roadmap](../../docs/roadmap/roadmap.md), and [active plans](../../PLANS.md) for current guidance.

## Goal

Complete the adjacent Phase 6–8 foundation as one coherent batch: deterministic Ground Truth event provenance, physically bounded sensory acquisition, generic feature extraction, and the minimal bounded attention primitive required by `TODO-COG-001`.

## Context

Phases 3–5 established causal geography, biological structure, and immutable pathogen exposure contracts. Those domains already reference `TraceId`, but no authoritative trace graph exists. The next two roadmap phases require a structural boundary between Ground Truth and cognition: only physically accessible signals may reach extraction, and authoritative `EntityId` values must stop before subjective scene construction. The existing cognition placeholder stores `Option<u64>` and does not provide bounded or deterministic attention.

## Relevant invariants

- INV-001, INV-002: acquisition is incomplete and structurally distinct from Ground Truth.
- INV-005, INV-006: no semantic event, signal, feature, or attention labels become authoritative.
- INV-014, INV-019, INV-026: accepted changes retain traversable causal support.
- INV-016: event commits follow deterministic proposal/reduce/commit ordering.
- INV-017, INV-018: layouts are batch-oriented and no scale claim is made without benchmarks.
- INV-027 through INV-031: authoritative identity stops before cognition and subjective detail requires accessible ancestry.

## Ontology domains affected

- Core simulation: event identity, state-change fingerprints, causal edges.
- Physical access: property-based signal channels and sensor apertures.
- Perception: generic features extracted from acquired samples only.
- Cognition: bounded subjective attention targets with no authoritative identity.

## Causal carriers affected

- Physical signals become explicit carriers between Ground Truth and sensory acquisition.
- `TraceId` edges preserve the causal ancestry of acquired samples and extracted features.
- Attention candidates retain supporting subjective `PerceptId` references. Phase 9 external bookkeeping will connect percepts to causal traces without exposing Ground Truth provenance to cognition.

## Relevant documents

- `docs/architecture/invariants.md`
- `docs/architecture/determinism.md`
- `docs/architecture/data-oriented.md`
- `docs/architecture/cognition-rebaseline.md`
- `docs/ontology/causal-carriers.md`
- `docs/ontology/primitive-vs-emergent.md`
- `docs/simulation/perceptual-features.md`
- `docs/cognition/attention.md`
- `docs/rfc/RFC-ONTO-001.md`
- `docs/rfc/RFC-COG-001.md`

## Current state

`TraceId` and `EventId` exist as passive IDs. Terrain generation and pathogen exposures retain trace references, but there is no trace store. `Feature` is a Phase 2 passive Ground Truth extractor record. There is no sensory acquisition crate or feature extraction algorithm. `AttentionState` is an unbounded semantic hole represented by `Option<u64>`.

## Proposed architecture

1. Add opaque typed schema IDs for event kinds, state targets, signal channels, sensors, acquisitions, and subjective attention targets.
2. Add a core structure-of-arrays causal event store. Proposals are ordered by stable numeric proposal keys, causes must already be committed, effects are property-level before/after fingerprints, and commits receive monotonic event/trace IDs.
3. Add `causafera-perception`. Phase 7 acquisition filters physical signals by channel, range, magnitude threshold, and time, then emits canonically ordered relative samples retaining input traces.
4. Phase 8 extraction consumes only acquired samples and emits generic magnitude/change features plus flattened causal input spans.
5. Replace the cognition placeholder with fixed-capacity attention state keyed only by subjective `AttentionTargetId`. Candidate ranking is deterministic and continuity is numeric rather than semantic.

## Primitive vs emergent review

Opaque numeric schema identity, physical position, signal magnitude, range, state fingerprints, generic feature relations, and bounded attention weights are primitives. Event names, sensory modality names, object categories, threats, opportunities, diseases, symptoms, and situation labels are not stored. Human labels remain observer/explanation metadata outside this work.

## Non-goals

- Domain-specific mutation systems or live infection progression.
- A full causal query engine or observer protocol expansion.
- Semantic sensory modalities, object recognition, concepts, beliefs, or subjective scene construction.
- Physically complete wave propagation, occlusion, physiology, or sensor damage.
- Frequency, synchrony, recurrence, or structural-similarity algorithms.
- Benchmark or scale claims.

## Implementation stages

1. Introduce IDs, provenance records, proposal validation, canonical commit, and traversal tests.
2. Introduce the perception crate and deterministic physical-access filtering.
3. Add generic extraction with flattened provenance.
4. Add fixed-capacity attention primitives and deterministic ranking.
5. Update RFCs, subsystem docs, roadmap, TODO backlog, and changelog.
6. Run workspace formatting, tests, clippy, diff validation, and graph reindexing.

## Verification

- Focused unit tests for invalid/non-canonical provenance input, deterministic proposal reduction, parent/child traversal, physical filtering, input-order independence, feature ancestry, attention bounds, and identity separation.
- `cargo fmt --all -- --check`
- `cargo test --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `git diff --check`

## Benchmark plan

No performance claim is introduced. After `TODO-PERF-001`, benchmark proposal sorting and commit throughput, bytes per causal event/edge, acquisition filtering throughput, features per sample, and attention update cost at declared batch sizes.

## Determinism impact

All authoritative ordering uses explicit numeric keys and stable sorting. No hash iteration, locale, system time, pointer identity, floating random source, or implicit parallel order participates. Feature scalar conversion follows the already accepted Phase 2 `f64` contract and is deterministic on the same reference hardware.

## Memory impact

The causal store and extracted-feature provenance use flat vectors with offsets; reverse causal children are a cold deterministic side index. Sensory and feature batches are contiguous. Per-agent attention uses fixed-size arrays and a hard candidate bound.

## Observer impact

No wire schema changes. Observer causal projection remains future work; the new store exposes read-only numeric event views suitable for a later adapter.

## Explanation impact

The trace graph supplies future explanation support but includes no glossing, confidence prose, or narrative rendering.

## Persistence impact

No snapshot schema is defined. New types derive only the traits required by current in-memory use. Persistence integration remains blocked on `TODO-PERSIST-001`.

## Cross-domain effects

Terrain, biology, physics, and future domain systems may use the core event store. Only `causafera-perception` bridges Ground Truth samples into extractor input. Cognition receives subjective attention targets, not acquired samples or authoritative entity identities.

## Risks

- Numeric schema IDs require a future registry/persistence policy; this phase deliberately does not invent human-readable authoritative labels.
- Simple range/threshold acquisition is a minimum physical-access boundary, not a claim of realistic sensing.
- Existing `Feature.target_id` remains authoritative at the extractor layer; Phase 9 must map it to perceived identity before cognition consumes it.
- Reverse-edge storage is a cold side index and must be benchmarked before scale claims.

## Documentation changes

Add accepted Phase 6 and Phase 7–8 RFCs; update roadmap, coverage matrix, primitive/emergent inventory, causal carriers, attention/perceptual-feature docs, and changelog.

## TODO changes

Complete `TODO-TRACE-001` and `TODO-COG-001`. Add and complete explicit Phase 7 sensory-access and Phase 8 feature-extraction entries so the roadmap scope is represented in the backlog.

## Decision log

- 2026-07-12: Batch Phases 6–8 because each downstream boundary depends directly on the preceding one and can be verified without Phase 9 semantics.
- 2026-07-12: Use opaque typed IDs and fingerprints instead of event-type strings or semantic enums.
- 2026-07-12: Permit `EntityId` only through the Ground Truth acquisition/extractor layers, consistent with the accepted cognition rebaseline; cognition attention uses a distinct subjective ID.
- 2026-07-12: Keep acquisition physics minimal and property-based; defer occlusion, propagation media, and biological sensor physiology.

## Progress

- [x] Required architecture, ontology, cognition, roadmap, ADR, and RFC context reviewed.
- [x] Phase 6 causal provenance implemented.
- [x] Phase 7 sensory acquisition implemented.
- [x] Phase 8 generic extraction implemented.
- [x] Bounded attention primitive implemented.
- [x] Documentation and TODO state updated.
- [x] Workspace verification and graph refresh completed.
