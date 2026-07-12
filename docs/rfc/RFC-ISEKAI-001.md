# RFC-ISEKAI-001: Cross-World Transfer and Imported Priors

**Status:** Accepted

## Summary

Phase 22 defines a bounded deterministic proposal/receipt contract for cross-world transfer without choosing a final identity metaphysics. It also separates imported subjective priors from independently evidenced local capability.

## Transfer contract

`CrossWorldTransferPlan` identifies a crossing by typed ID, explicit seed, opaque mechanism schema, simulation time, opaque source-world identity and location fingerprint, objective target place, canonical payloads, property correspondences, and prior causal traces.

Mechanism schemas are registered binary adapter identities. They are not a semantic enum of physical transfer, reincarnation, memory transfer, artifact arrival, or souls. Payloads contain an opaque objective object-kind schema and canonical source-state fingerprint. Property correspondences state which source property is represented by which target property; an absent correspondence makes no persistence claim.

The plan is proposal-only. A concrete adapter remains responsible for phase-controlled READ → PROPOSE → REDUCE → COMMIT. Its receipt must occur no earlier than scheduled, exactly cover every payload in canonical order, and exactly continue the plan's causal traces. The resulting immutable record carries the committed trace.

## Determinism and bounds

Execution seed contribution depends only on the explicit seed, transfer identity, mechanism identity, and scheduled time through integer-only mixing. Payloads, correspondences, causes, priors, requirements, and evidence have hard caps and canonical typed-ID order. No entropy, locale, strings, floating point, pointer identity, hash iteration, or scheduling order participates.

## Imported priors

`ImportedPriorBundle` is linked to a committed transfer trace and contains subjective pattern IDs, initial cognitive weights, and subjective source IDs. A prior is not Ground Truth, a concept automatically installed in another agent, or evidence that its content is locally true.

`ReproductionRequirements` separately names required practices, materials, resource schemas, and measurement schemas. `CapabilityEvidence` supplies independently acquired local evidence and causal traces. Gap assessment is read-only and purely structural. Imported prior membership cannot satisfy any prerequisite.

This enforces INV-025: knowing or expecting something does not supply embodiment, procedure, tools, materials, measurement, credibility, or social transmission.

## Primitive and emergent boundary

Typed identity, time, place, fingerprints, property correspondence, source/target bookkeeping, weights, prerequisite membership, and causal traces are primitive. Transfer interpretations, personal continuity, souls, copies, reincarnation, hero status, truth, technology, usefulness, social response, and historical significance are unresolved hypotheses or emergent classifications.

## Observer and explanation impact

No observer protocol changes are made. A future read-only projection may gloss mechanism schemas and display arrival chronology, but labels remain non-authoritative. Explanations may traverse receipt and capability-evidence traces and must expose missing prerequisites instead of narratively granting competence.

## Persistence impact

No snapshot format is selected. Future persistence must preserve schema versions, canonical order, fingerprints, source/target identities, weights, and trace identities exactly.

## Decisions

- **Accepted:** opaque transfer mechanism schemas and objective payload/property carriers.
- **Accepted:** explicit deterministic seed inputs and exact receipt ancestry/coverage.
- **Accepted:** imported patterns are subjective priors linked to transfer provenance.
- **Accepted:** capability requires separate local practice, material, resource, and measurement evidence.
- **Deferred:** final metaphysics, concrete transfer physics, identity continuity, body binding, cognition mutation, translation, persistence, observer projection, and Phase 23 experiments.
