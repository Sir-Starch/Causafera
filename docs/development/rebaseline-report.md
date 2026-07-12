# Architecture Rebaseline Final Report

> Historical audit note: this report captured readiness immediately after the cognition rebaseline. Phases 6–24 were subsequently completed on 2026-07-12; the roadmap and TODO backlog are authoritative for current status. RFC-GEO-002 later corrected a pre-existing ambiguity by separating finite charted global geography, bounded local 3D geometry, containment, and causal resolution without invalidating the completed subjective-scene boundary.

## 1. Which completed phases were audited?

Phases 1 through 5 were audited:

- **Phase 1:** Deterministic simulation kernel (`ontopolis-core` — scheduler, phases, random streams, deterministic config)
- **Phase 2:** Ontology primitives and generic feature representation (`ontopolis-types` — coordinates, physics, features, IDs)
- **Phase 3:** Spatial world skeleton (`ontopolis-world` — hierarchy, spatial containment)
- **Phase 4:** Minimal causal geography (`ontopolis-geography` — terrain contracts, generation, provenance)
- **Phase 5:** Biological structural model (`ontopolis-biology` — body segments, joints, lengths, orientations)

## 2. Were Phases 1–5 preserved as completed?

Yes. Phases 1–5 remain completed with their original numbers. No completed work was renumbered or obscured. The rebaseline document explicitly states that the previous architecture was not invalid and that Phases 1–5 are structurally sound.

## 3. What cognitive architecture gap was formally recorded?

The missing **subjective scene construction layer** between generic perceptual feature extraction and subjective concept/belief formation was formally recorded.

The previous architecture described a direct progression:

```text
Ground Truth → Physical access → Generic features → Concepts → Beliefs
```

It did not explicitly describe how an agent constructs a coherent, transient, agent-specific model of the currently experienced situation from scattered perceptual input. The rebaseline inserts the subjective scene layer:

```text
Ground Truth → Physical access → Generic features → Subjective Scene → Concepts → Beliefs
```

## 4. Which new invariants were added?

Nine new invariants were appended to `docs/architecture/invariants.md` using the existing numbering scheme:

- **INV-027:** Agents do not directly perceive authoritative entity identity
- **INV-028:** Perceived object identity is a subjective hypothesis
- **INV-029:** Agents act on a constructed subjective scene
- **INV-030:** Subjective scene content must be causally grounded
- **INV-031:** Subjective detail cannot introduce inaccessible information
- **INV-032:** Persistent autobiographical memory is not continuously active context
- **INV-033:** The self-model is subjective
- **INV-034:** Objective body state and subjective body schema are distinct
- **INV-035:** Prediction error is a first-class cognitive driver

## 5. Where is the cognition rebaseline documented?

- **`docs/architecture/cognition-rebaseline.md`** — The primary architecture rebaseline document: "Cognition Rebaseline: Subjective Scene and Cognitive Continuity"
- **`docs/architecture/invariants.md`** — New invariants INV-027 through INV-035
- **`docs/roadmap/roadmap.md`** — Updated phase sequence
- **`docs/development/todo-backlog.md`** — New TODO items and updated dependencies
- **`AGENTS.md`** — Cognitive architecture constraints for future AI agents

## 6. What questions does RFC-COG-001 leave open?

RFC-COG-001 (`docs/rfc/RFC-COG-001.md`) leaves ten explicit unresolved questions:

1. Minimum viable Rust type layout for `SubjectiveScene` that satisfies bounded-size constraints.
2. How episodic memory similarity matching scales and what indexing structure is needed.
3. How prediction error propagates across subsystems without a global update cascade.
4. Correct balance between sparse reactivation and memory loss for inactive agents.
5. How the subjective scene interacts with existing `AttentionState` and `SalienceState`.
6. Whether `PerceivedObjectIdentity` should be a typed ID or a more complex structure.
7. How the body schema updates when biological state changes (injury, growth, fatigue).
8. Minimum viable representation for a "situation prototype" driving prediction and concept formation.
9. How to unit-test subjective scene construction without building a full agent.
10. Performance budget for subjective scene reconstruction per agent per tick.

## 7. Did any existing code violate the new invariants?

No existing code in Phases 1–5 directly violates the new invariants because no agent cognition has been implemented yet. The audit found:

- `Feature.target_id: EntityId` in `ontopolis-types` creates a **future risk** — if future cognition consumes `Feature` directly, it would receive authoritative entity identity. However, no code currently consumes `Feature` in the cognition crate.
- `BodyStructure` with `BodySegmentId` is not imported by `ontopolis-cognition`.
- `SpatialHierarchy` with `PlaceId`/`ChunkId` is not imported by `ontopolis-cognition`.
- No semantic shortcuts (`Finger`, `Tremor`, `Disease`, etc.) exist in the generic feature layer.

## 8. If code changed, what exact incompatibility required it?

No code changes were made. The audit determined that documentation and future-contract warnings were the appropriate response because:

- No existing agent code consumes `Feature.target_id` or `BodyStructure` directly.
- The invariants target agent behavior, and no agents exist in Phases 1–5.
- Changing `Feature` now would be speculative; the precise type layout is intentionally left to RFC-COG-001.

## 9. Does authoritative identity remain distinct from future perceived identity?

Yes. The architecture explicitly requires this separation:

- **INV-027** forbids agents from directly perceiving `EntityId`, `BodySegmentId`, or `PlaceId`.
- **INV-028** states that perceived object identity is a subjective hypothesis that may be wrong.
- **RFC-COG-001** defines `PerceivedObjectIdentity` as structurally distinct from `EntityId`. Agent cognitive state does not contain authoritative ID guesses; continuity and identity are expressed through `same_as_previous_confidence`, `appearance_signature`, and `relationship_associations`.
- **TODO-SCENE-002** is dedicated to implementing perceived object persistence with identity-error propagation.

## 10. Does biological Ground Truth remain distinct from future body schema?

Yes. The separation is architecturally enforced:

- **INV-034** explicitly separates objective body state from subjective body schema.
- `ontopolis-biology` stores `BodyStructure` with `BodySegmentId`, length, orientation, and joint limits.
- `ontopolis-cognition` does not currently import `ontopolis-biology`.
- **RFC-COG-001** defines `BodySchemaState` as constructed from proprioception, pain, balance, and learned boundaries — not from a complete `BodyStructure` dump.
- **TODO-SCENE-003** is dedicated to designing the body-schema mapping.

## 11. Were any hidden semantic primitives found in the generic feature layer?

No. The audit searched aggressively for semantic primitives in the feature layer and found none:

- `FeatureRelation` contains only genuinely generic relations: `Change`, `Magnitude`, `Direction`, `Variance`, `Periodicity`, `Synchrony`, `Recurrence`, `Duration`, `SpatialRelation`, `TemporalRelation`, `CoOccurrence`, `StructuralSimilarity`, `RelativeDifference`, `SequenceSimilarity`.
- No `Finger`, `Tremor`, `Disease`, `EmotionKind`, `Furniture`, `Profession`, `Skill`, `Class`, `Level`, `Monster`, or `Sacred` types exist in `*.rs` files outside of the Explanation Engine and observer analytics (which are non-authoritative by design).
- `docs/ontology/primitive-vs-emergent.md` explicitly documents the boundary and confirms Phase 2 primitives are property-based, not taxonomy-based.

## 12. How was the roadmap changed?

Phases 1–5 remain unchanged. Phases 6–27 were resequenced to insert the subjective scene and cognitive continuity layers before concept formation and belief:

| Old Phase | New Phase | Description |
|-----------|-----------|-------------|
| 7 | 7 | Physical access and sensory acquisition |
| 8 | 8 | Generic perceptual feature extraction |
| 9 | 9 | **Subjective Scene Construction** |
| 10 | 10 | **Working context, prediction, and cognitive continuity** |
| 11 | 11 | **Sparse subjective concept formation** |
| 12 | 12 | **Beliefs and subjective causal inference** |
| 13 | 13 | Language bootstrap |
| 14 | 14 | Lexical innovation |
| 15 | 15 | Practice representation |
| 16 | 16 | Measurement and epistemics |
| 17 | 17 | Minimal mana |
| 18 | 18 | Causal Resolution Field |
| 19 | 19 | Social networks |
| 20 | 20 | Material economy |
| 21 | 21 | Historical bootstrap |
| 22 | 22 | Isekai transfer |
| 23 | 23 | Metaphysical experiments |
| 24 | 24 | Long-run emergence |
| — | 25 | Explanation Engine expansion |
| — | 26 | Rich observer UI |
| — | 27 | Optional narrative surface |

## 13. How was the TODO dependency graph changed?

**New TODO items added (9):**

- `TODO-SCENE-001` — Implement minimal Subjective Scene representation (Phase 9, depends on `RFC-COG-001: Accepted`)
- `TODO-SCENE-002` — Perceived Object Persistence (Phase 9, depends on `TODO-SCENE-001`)
- `TODO-SCENE-003` — Subjective Body Schema (Phase 9, depends on `TODO-BIO-001`, `TODO-SCENE-001`)
- `TODO-SCENE-004` — Self-Model Architecture (Phase 9, depends on `TODO-SCENE-001`)
- `TODO-SCENE-005` — Predictive World Model (Phase 10, depends on `TODO-SCENE-001`)
- `TODO-SCENE-006` — Working Memory and Active Context (Phase 10, depends on `TODO-SCENE-001`)
- `TODO-SCENE-007` — Episodic Memory Reactivation (Phase 10, depends on `TODO-SCENE-006`)
- `TODO-SCENE-008` — Agency Attribution (Phase 10, depends on `TODO-SCENE-005`)
- `TODO-SCENE-009` — Subjective Temporal Continuity (Phase 10, depends on `TODO-SCENE-001`)

**Dependencies updated to block premature concept/belief work:**

- `TODO-CONCEPT-001` (sparse concept formation) now depends on `TODO-COG-001`, `TODO-SCENE-001`, and `TODO-SCENE-006`.
- `TODO-COG-002` (bounded cognition / belief) now depends on `TODO-COG-001` and `TODO-SCENE-006`.

**Phase numbers updated for 20 existing TODOs** to match the rebaselined roadmap (e.g., `TODO-CONCEPT-001` moved from Phase 8 to Phase 11, `TODO-LANG-001` from Phase 10 to Phase 13, etc.).

## 14. Which future task is now the first READY task?

At the time of this audit, the first pending (READY-to-begin) task was **`TODO-TRACE-001: Causal Provenance System`** (Phase 6).

At the time of this audit, all Phase 6 work was unblocked: it depended only on `TODO-CORE-001` (completed). The next sequential task was `TODO-COG-001: Attention Primitives` (Phase 7), depending on `TODO-BIO-001` (completed).

## Subsequent implementation status (2026-07-12)

RFC-COG-001, RFC-SCENE-001, RFC-CONCEPT-001, RFC-LANG-001, RFC-LANG-002, RFC-PRACTICE-001, RFC-EPI-001, RFC-MANA-001, RFC-RES-001, RFC-SOCIAL-001, RFC-ECON-001, RFC-CITY-001, RFC-HIST-001, and RFC-ISEKAI-001 are now accepted. `TODO-SCENE-001` through `TODO-SCENE-009` were completed in the Phase 9–10 batch; `TODO-CONCEPT-001` and `TODO-COG-002` were completed in the Phase 11–12 batch; `TODO-LANG-001` through `TODO-LANG-003` were completed in the Phase 13–14 batch; `TODO-PRACTICE-001`, `TODO-EPI-001`, and `TODO-LANG-004` were completed in the Phase 15–16 batch; `TODO-MANA-001` was completed in Phase 17; `TODO-RES-001` was completed in Phase 18; `TODO-SOCIAL-001` and `TODO-SOCIAL-002` were completed in Phase 19; `TODO-ECON-001` and `TODO-CITY-001` were completed together in Phase 20; `TODO-HIST-001` was completed in Phase 21; `TODO-ISEKAI-001` and `TODO-ISEKAI-002` were completed together in Phase 22. Phase 23 is now next; this historical report otherwise preserves the findings that motivated the rebaseline.

## 15. What was deliberately not implemented?

The following were explicitly excluded from this rebaseline task, per the instructions:

- **No new Rust structs** for `SubjectiveScene`, `PerceivedObjectIdentity`, `BodySchemaState`, `SelfModel`, `WorkingContext`, or `TemporalEnvelope`.
- **No runtime storage** for perceived identities, memory systems, prediction systems, or active context.
- **No fake agents** or placeholder subjective scenes.
- **No fake cognitive systems** — no `AnxietySituation` enum, no `RememberFatherMoment` event, no semantic situation shortcuts.
- **No code changes** to existing Phase 1–5 contracts, because no direct invariant violations were found.
- **No reimplementation** of any completed phase.
- **No beginning** of the next implementation phase. The task stopped after architecture rebaseline, compatibility audit, documentation/TODO/roadmap updates, verification, and this report.

## Verification Summary

| Check | Result |
|-------|--------|
| `cargo fmt --check` | Pass (no output) |
| `cargo clippy --workspace --all-targets --all-features` | Pass (no warnings) |
| `cargo test --workspace --all-features` | Pass (64 tests passed, 0 failed) |
| `cargo test --workspace --no-default-features` | Pass (64 tests passed, 0 failed) |
| Stale roadmap phase references | Updated in 12 RFCs, domain coverage matrix, and TODO backlog |
| Stale TODO dependency references | Updated 20 existing TODO phase numbers; added 9 new TODOs with blocking dependencies |
| Contradictory cognition documentation | No contradictions found; new docs are consistent with existing vision and invariants |
| Hidden semantic feature enums | None found in generic feature layer |
