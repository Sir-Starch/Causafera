# RFC-PERCEPT-001: Physical Access, Generic Extraction, and Attention Boundary

**Status:** Accepted

## Summary

Define the minimal Phase 7–8 pipeline from Ground Truth physical signals through bounded acquisition and generic feature extraction, plus a bounded subjective attention primitive that cannot store authoritative entity identity.

## Motivation

INV-001 and INV-002 require structurally incomplete sensory access. INV-027 through INV-031 require authoritative identity and inaccessible information to stop before cognition. Phase 2 defined passive generic `Feature` values, but no process ensured that extractors operated only on physically accessible state. `TODO-COG-001` also had only an `Option<u64>` placeholder with no bound, salience rule, or identity separation.

## Phase 7 physical access

`PhysicalSignal` contains an objective source, opaque `SignalChannelId`, world position, signed quantized magnitude, simulation time, and causal trace. `SensorAperture` contains an opaque sensor/channel identity, owner, world position, integer range, and minimum absolute magnitude.

`acquire_signals` emits a sample only when time and channel match and the signal passes threshold and Chebyshev-range checks. Samples store relative rather than absolute position and retain the source trace. Inputs are canonicalized; acquisition IDs are assigned after sorting.

Signal channels are physical schema identities, not semantic modality enums such as sight, hearing, smell, or pain. Occlusion, propagation media, biological transduction, adaptation, damage, and noise are deferred. The current rule is a minimum access boundary, not a realism claim.

## Phase 8 generic extraction

`GenericFeatureExtractor` accepts only `SensorySample`, never raw Ground Truth. It canonicalizes samples and emits:

- `Magnitude` with a configured property-based magnitude quantum;
- `Change` between consecutive samples for the same sensor, source, and channel.

Each feature retains a flattened ordered span of supporting input traces. Frequency, periodicity, synchrony, recurrence, duration, spatial relation, and similarity algorithms remain future extensions under the same non-semantic boundary.

`Feature.target_id` remains authoritative extractor bookkeeping, as explicitly permitted by the accepted cognition rebaseline. Phase 9 must map it to a subjective perceived identity before cognition consumes the feature.

## Phase 7 attention primitive

`AttentionState` has a fixed maximum of eight active foci and accepts no more than 64 candidates per update. Candidates contain only an agent-local `AttentionTargetId`, fixed-point salience, and a supporting subjective `PerceptId`. They cannot contain `EntityId`, `BodySegmentId`, `PlaceId`, `FeatureId`, or `TraceId`.

Selection filters by threshold, adds a numeric continuity bonus to current foci, sorts by descending effective weight, and breaks ties by subjective target ID. Active-since time is preserved when focus continues. Input ordering, hash iteration, and randomness do not affect the result.

This attention primitive does not yet consume extractor features. Phase 9 subjective scene mapping must create grounded subjective targets and maintain inaccessible external percept-to-trace bookkeeping before attention and later cognition use them.

Phase 9 follow-up is now implemented by the identity-free `PerceptualCue` boundary accepted in RFC-SCENE-001. The inaccessible percept-to-trace correspondence remains outside cognition.

## Primitive and emergent boundary

Physical magnitude, position, range, threshold, channel schema identity, generic relation/value, fixed-point attention weight, and subjective target identity are permitted. Object categories, modality names, threat/opportunity labels, symptoms, emotions, situations, and concepts are not authoritative values in this pipeline.

## Determinism and memory

All ordering uses explicit typed numeric keys and stable sorting. Acquisition and extracted features are contiguous batches. Feature provenance uses flat offsets. Attention active state uses fixed arrays. No performance claim is made; workloads must be benchmarked after `TODO-PERF-001`.

## Observer and explanation impact

No wire or explanation schema changes. Future observer adapters may expose derived acquisition, feature, or attention diagnostics subject to Ground Truth/subjective separation and confidence requirements.

## Non-goals

- Realistic wave, optical, acoustic, chemical, neural, or physiological sensing.
- Object recognition, perceived identity persistence, body schema, subjective scene, concepts, or beliefs.
- Semantic sensor, feature, attention, emotion, threat, or situation taxonomies.
- Continuous extraction over all world state.
- Benchmark or emergence claims.

## Decision log

- **Accepted:** Create `causafera-perception` as the explicit authoritative acquisition/extraction boundary.
- **Accepted:** Acquisition uses minimal integer channel/range/threshold filtering and relative samples.
- **Accepted:** Extractors accept only acquired samples and retain causal input traces.
- **Accepted:** Authoritative source identity may exist through extraction but must be mapped before cognition.
- **Accepted:** Attention targets are distinct agent-local subjective IDs with fixed capacity and deterministic ranking.
