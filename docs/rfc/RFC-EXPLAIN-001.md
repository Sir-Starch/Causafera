# RFC-EXPLAIN-001: Observer Analytical Ontology and Explanation IR

**Status:** Accepted

## Summary

The Explanation IR is a read-only, deterministic structure for observer-side analytical claims about simulation experiments. It converts checkpoints, causal trace references, and numeric analytics into typed claim frames that can be rendered for humans without mutating authoritative simulation state or introducing English semantic categories as simulation meaning.

## Motivation

Causafera needs causal explanations that are inspectable, reproducible, and honest about uncertainty. A checkpoint or intervention result must not be rendered as an emergent phenomenon merely because a metric exists. Each explanation claim therefore carries:

- an opaque typed schema ID;
- a typed numeric value or numeric range;
- a confidence in `[0.0, 1.0]`;
- supporting `TraceId` references from the causal trace store;
- a comparison context when the claim depends on a cohort, control, or counterfactual;
- an explicit supported, unsupported, or unknown evidence state.

## Non-authoritative boundary

The Explanation IR is downstream of authoritative simulation and analytics.

```text
AUTHORITATIVE CHECKPOINTS / TRACE STORE
    ↓
DETERMINISTIC ANALYTICS
    ↓
EXPLANATION IR
    ↓
DETERMINISTIC RENDERING
    ↓
OPTIONAL LOCALIZED OR LLM SURFACE
```

The IR never feeds back into simulation phases, agent cognition, mana response, canonical digests, or authoritative history. Observer strings, localization, and UI labels are allowed only after the structured IR exists and must not affect the IR's deterministic ordering or values.

## Core schema

### `ExplanationClaim`

An `ExplanationClaim` is one typed numeric assertion.

Required fields:

| Field | Type | Meaning |
| --- | --- | --- |
| `schema` | `ExplanationClaimSchemaId` | Opaque externally registered claim schema. It is not an English classification. |
| `value` | `NumericClaimValue` | Scalar or closed numeric range. No string claim values are allowed. |
| `confidence` | `ClaimConfidence` | Bounded numeric confidence in `[0.0, 1.0]`. |
| `evidence_traces` | ordered unique `TraceId` set | Causal trace references supporting the claim. |
| `comparison` | `ComparisonContext` | Cohort/control/counterfactual context, or explicit absence of comparison. |
| `evidence_state` | `ClaimEvidenceState` | `Supported`, `Unsupported`, or `Unknown`. |

`NumericClaimValue` variants:

- `Scalar { value: i64 }`
- `Range { start: i64, end: i64 }`
- `Ratio { numerator: u64, denominator: u64 }` where denominator must be non-zero

`ClaimEvidenceState` variants:

- `Supported` — supporting traces are present and confidence may be non-zero.
- `Unsupported` — required evidence is absent or invalid; confidence must be zero.
- `Unknown` — the analytical question was not answerable from available checkpoints; confidence must be zero.

Removing or changing evidence must deterministically lower confidence or transition the claim to `Unsupported`/`Unknown`. A supported claim with no trace references is invalid.

### `ComparisonContext`

Comparison context is structured, not prose:

- `None` — claim is an absolute observation.
- `MatchedCohort { cohort: ComparisonCohortId }` — claim compares matched checkpoints or matched subjects.
- `Counterfactual { cohort: ComparisonCohortId }` — claim depends on an explicit counterfactual or intervention series.

The cohort ID is opaque. Human-facing labels for “control”, “intervention”, or “same tick” belong to rendering metadata, not authoritative claim semantics.

### `ExplanationFrame`

An `ExplanationFrame` represents all claims for one checkpoint time.

Required fields:

| Field | Type | Meaning |
| --- | --- | --- |
| `checkpoint_time` | `SimulationTime` | Checkpoint tick. |
| `claims` | ordered `ExplanationClaim` list | Deterministically ordered by schema ID and comparison context. |
| `overall_assessment` | `FrameAssessment` | Supported, partial, unsupported, or unknown state for the frame. |

`FrameAssessment` is derived from claim evidence states. It is not a narrative classification and must not be used as simulation state.

### `ExplanationReport`

An `ExplanationReport` represents a full experiment.

Required fields:

| Field | Type | Meaning |
| --- | --- | --- |
| `experiment` | `ExperimentId` | Experiment identity. |
| `frames` | ordered `ExplanationFrame` list | One or more checkpoint frames. |
| `overall_assessment` | `FrameAssessment` | Deterministic reduction of frame assessments. |

Reports must be deterministic: same seed, same checkpoints, same trace references, and same analytics inputs produce byte-for-byte equivalent IR values.

## Minimal Stage 2 claim schemas

Stage 2 reserves opaque numeric schema IDs for experiment analytics. The numbers are stable; the names below are RFC documentation only and must not appear as authoritative runtime classifications.

| Schema ID | Numeric meaning |
| --- | --- |
| `1` | reconstructability ratio from trace density |
| `2` | path-dependence ratio from seed sensitivity |
| `3` | causal depth |
| `4` | temporal span |
| `5` | counterfactual state distance |
| `6` | recovery distance to matched control |
| `7` | time-to-recovery |
| `8` | field stability under active input |
| `9` | field stability without active input |

These IDs describe numeric schemas, not human semantic categories. Rendering may attach localized text to schema metadata, but rendering text never changes claim identity or deterministic output.

## Checkpoint and intervention analytics

Matched checkpoint analysis compares an intervention series against a control series at identical ticks. It must use the separated Stage 1 digests:

- `PhysicalStateDigest` for exact physical identity/equality and divergence detection;
- `HistoryDigest` for historical/provenance divergence;
- `ExperimentDigest` only for whole-trajectory identity, not as a replacement for physical/history analysis.

### Detailed Development correction: identity is not distance

Digest bytes are avalanche-style identity summaries. Equality is meaningful; arithmetic distance
between their bytes is not. `PhysicalStateDigest` therefore cannot define physical proximity,
recovery distance, stability, counterfactual magnitude, or tolerance. The Foundation Era
digest-distance implementation is transitional diagnostic code and may not support a mature
`Supported` physical-recovery claim.

Physical comparison requires a versioned typed domain state vector or a set of typed property
deltas. Each component must declare its unit/scale, normalization or comparison rule, aggregation
rule, observation scope, and acceptable tolerance. Missing domain components must reduce coverage
or produce `Unknown`/`Unsupported`; they may not be replaced by digest arithmetic.

Required recovery outputs:

- pre-intervention baseline distance;
- perturbation minimum or maximum distance across the intervention window;
- matched control distance at identical ticks;
- final recovery distance;
- optional time-to-recovery when distance returns within tolerance after perturbation.

Every tolerance must be valid for the declared domain metric and must be smaller than or equal to
that metric's documented range unless an explicit unbounded scale is justified. Recovery requires
both a post-perturbation checkpoint and convergence under the same typed metric used for the
baseline. A checkpoint interval cannot itself be reported as discovered recovery without this test.

The distinction between driven equilibrium and autonomous persistence is evidence-based:

- driven equilibrium requires stable field metrics while active input is present;
- autonomous persistence requires stable field metrics after input has been removed;
- if checkpoints do not cover the relevant window or traces are missing, the claim must be `Unknown` or `Unsupported`.

## Determinism requirements

- Claim ordering is deterministic.
- Trace references are sorted and deduplicated.
- Confidence is a pure function of evidence state and trace density.
- Missing evidence cannot increase confidence.
- English labels, observer locale, renderer selection, and optional LLM output cannot affect claim values, confidence, digests, or ordering.

## Unresolved Questions

- Richer analytical ontology registration and metadata transport.
- Domain state-vector registration, scaling, and partial-coverage reduction.
- Optional LLM fact-packet boundary only after the terminal gate in the Detailed Development
  rebaseline; it is not current roadmap work.

## Implemented observer delivery

The accepted minimal renderer provides deterministic English and Russian templates for the
reserved Stage 2 numeric schemas, preserves evidence state, confidence, comparison context,
and trace counts, and uses an explicit generic fallback for unknown schema IDs. Observer v1
transports the typed IR separately from optional rendered prose.
