# Phases 13–14 Language Foundation

> **Historical record.** This completed ExecPlan describes a Foundation Era project stage. Its implementation status and terminology may be outdated; use [the documentation index](../../docs/index.md), [roadmap](../../docs/roadmap/roadmap.md), and [active plans](../../PLANS.md) for current guidance.

## Goal
Implement deterministic historical language bootstrap, bounded subjective lexicons, physical-form communication boundaries, communicative pressure, lexical innovation, and minimal adoption/semantic revision.

## Context
Phases 11–12 provide subjective concepts and beliefs. The language crate is currently a placeholder using strings, floating point weights, and an objective-looking map. Phases 13 and 14 share the same phonological, lexical-lineage, and exposure data model and are therefore implemented together.

## Relevant invariants
INV-006, INV-008, INV-014, INV-016, INV-027, INV-029, and INV-035. No human-language strings are authoritative meaning; speaker intent, physical form, and listener interpretation remain distinct.

## Ontology domains affected
Language and cognition. Language forms and transmission histories are causal state; semantic associations remain subjective cognition.

## Causal carriers affected
Abstract phonological units, ordered forms, lexeme lineages, utterance-form observations, contextual concept hypotheses, and transmission records.

## Relevant documents
`docs/language/*`, `docs/cognition/strategic-communication.md`, `docs/architecture/invariants.md`, `docs/architecture/determinism.md`, RFC-LANG-001, and RFC-LANG-002.

## Current state
`causafera-language` exposes only a `Vec<String>` phoneme inventory, a `HashMap<ConceptId, f32>` lexicon entry, and a string grammar field. It has no bootstrap, communication boundary, innovation, or history.

## Proposed architecture
Use bounded fixed-point records and opaque typed IDs. A language lineage owns a bounded abstract phoneme inventory and phonotactic constraints. Bootstrap derives canonical lineage and inherited form records from an explicit seed. Lexemes contain form lineage and use history but no meaning. Each subjective lexicon separately stores weighted concept associations. Communication emits an ordered physical form record; decoding receives only the observed form plus listener context. Repeated unmet concept-reference demand accumulates pressure and may deterministically coin a form. Exposure revises familiarity and semantic hypotheses; repeated adoption creates transmission records.

## Primitive vs emergent review
Opaque units, numeric weights, lineage links, ordered form units, and evidence references are primitives. Words' meanings, languages as communities, polysemy, synonymy, prestige, and semantic categories are emergent distributions. No semantic enums or gloss strings are introduced.

## Non-goals
Natural-language prose, full grammar/morphology, acoustic propagation, writing, social-network simulation, high-resolution historical conversations, and Phase 15 practices.

## Implementation stages
1. Replace placeholder phonology and lexicon types with bounded deterministic representations.
2. Add historical bootstrap and form-lineage inheritance.
3. Add communication, decoding, pressure, coinage, exposure, adoption, and drift primitives.
4. Add tests, accept both RFCs, update roadmap/TODO/domain documentation, and verify the workspace.

## Verification
Unit tests cover seed determinism, canonical input handling, phonotactic validity, absence of objective meaning, intent/utterance/interpretation separation, repeated pressure, deterministic coinage, bounded adoption, and exposure-driven association change. Run fmt, workspace tests, clippy with warnings denied, architectural searches, and diff checks.

## Benchmark plan
No scale or throughput claim. Fixed capacities and contiguous vectors establish measurable bounds; benchmarking remains TODO-PERF-001 work.

## Determinism impact
All generation uses explicit integer mixing from seed and opaque IDs. Inputs are canonicalized before allocation. No hash-map iteration, floating point, system entropy, locale, strings, or scheduler order affects state.

## Memory impact
All hot collections have documented hard maxima. Forms use compact unit IDs; semantic associations and histories are sparse bounded vectors.

## Observer impact
No wire mutation in this phase. Future read models may project lineage trees, form histories, pressure, and subjective distributions as derived analytics.

## Explanation impact
Lineage, transmission, exposure, and interpretation evidence make coinage and misunderstanding inspectable. Explanation remains read-only.

## Persistence impact
No snapshot-format change. New records are serialization-ready but not wired into persistence.

## Cross-domain effects
Consumes subjective ConceptIds without importing complete concept state. Future physical acoustics, social networks, documents, and mana may consume language outputs through explicit boundaries.

## Risks
The minimal phonotactic model may be too coarse for later morphology; opaque unit IDs mitigate replacement cost. Bootstrap is structural rather than a claim of historical realism. Community adoption is evidence-driven but does not yet simulate a social graph.

## Documentation changes
Accept RFC-LANG-001/002; update language subsystem docs, ontology matrices, roadmap, changelog, rebaseline report, and index as needed.

## TODO changes
Complete TODO-LANG-001, TODO-LANG-002, and TODO-LANG-003. Leave writing and practices pending.

## Decision log
- 2026-07-12: Batch Phases 13–14 because bootstrap, innovation, and semantic learning require one shared form/lineage representation.
- 2026-07-12: Model phonological units as opaque numeric categories; human glyphs and IPA are observer-only renderings.
- 2026-07-12: Keep subjective lexicons out of lexeme lineage records, preventing objective meaning fields.

## Progress
- [x] Architecture and scope established.
- [x] Implementation complete.
- [x] Verification complete.
- [x] Documentation and TODO complete.
