# Causafera Roadmap

## Phase 0

Complete project foundation.

Completed.

## Phase 1

Deterministic simulation kernel.

Completed.

## Phase 2

Ontology primitives and generic feature representation.

Do not implement semantic concepts yet.

Completed.

## Phase 3

Spatial world skeleton.

Completed.

## Phase 4

Minimal causal geography.

Completed.

## Phase 5

Biological structural model and immutable pathogen contracts.

Completed.

## Phase 6

Ground Truth events and causal provenance.

Completed.

## Phase 7

Physical access, sensory acquisition, and bounded attention primitives.

Completed.

## Phase 8

Generic perceptual feature extraction.

Completed.

## Phase 9

Subjective Scene Construction.

Completed.

## Phase 10

Working context, prediction, and cognitive continuity.

Completed.

## Phase 11

Sparse subjective concept formation.

Completed.

## Phase 12

Beliefs and subjective causal inference.

Completed.

## Phase 13

Language bootstrap and communication architecture.

Completed.

## Phase 14

Lexical innovation, semantic inference, and language change.

Completed.

## Phase 15

Practice representation and evolution.

Completed.

## Phase 16

Measurement, documents, and epistemic infrastructure.

Completed.

## Phase 17

Minimal information-sensitive mana.

Completed.

## Phase 18

Causal Resolution Field.

Completed.

## Phase 19

Social networks and organizations.

Completed.

## Phase 20

Material economy and city infrastructure.

Completed.

## Phase 21

Historical bootstrap.

Completed.

## Phase 22

Isekai transfer and imported priors.

Completed.

## Phase 23

Metaphysical experiments and attractors.

Completed.

## Phase 24

Long-run emergence experiments.

Completed. The delivered scope is a replay-verified bounded causal field experiment and does not claim that semantic phenomena emerged.

RFC-GEO-002 and `TODO-GEO-003` were completed as a foundational correction discovered before final Phase 24 verification: local physics is full 3D, global geography is a finite charted 2.5D planetary surface with selective volumetric 3D, and containment remains separate from geometry.

## Phase 25

Explanation Engine expansion. Minimal typed IR, matched experiment claims, deterministic localized
rendering, and observer v1 transport are complete; richer analytical ontology remains incremental.

A minimal Explanation IR may exist earlier for developer inspection.

Completed within the Foundation Era scope. Digest-distance recovery and the minimal analytical
ontology are explicitly not considered mature Detailed Development analytics.

## Phase 26

Rich observer UI consuming only the versioned observer protocol. Direct runtime storage access is
forbidden.

Completed. The Tauri 2 desktop application runs a causally bootstrapped runtime, negotiates
observer v1, consumes digest-anchored bounded runtime streams, queries chart-qualified world chunk
projections, and renders typed Explanation IR. The delivered UI provides World, causal-loop,
timeline, inspector, and comparative-explanation views in deterministic presentation across five
locales (`en-US`, `ru-RU`, `zh-Hans`, `de-DE`, `es-ES`), covering UI chrome, locale-keyed observer
metadata, and the authoritative Explanation renderer alike. Rich entity, language, society,
historical comparison, and large-dataset WebGPU views remain incremental observer work.

## Foundation Era boundary

Phase 26 is the last preallocated Foundation Era phase. Completion of Phases 0–26 means Causafera
has minimum valid domain contracts, deterministic execution, causal provenance, persistence, a
bounded long-run harness, Explanation IR transport, and a real observer UI. It does not mean that
the domains have reached full causal depth or that semantic emergence has been demonstrated.

## Detailed Development Program — current

Causafera is now in open-ended detailed development. The final number of phases is deliberately
unknown. New numbered phases or implementation batches are allocated only when bounded ExecPlans
are accepted; the roadmap does not reserve a terminal phase in advance.

Normative priority:

1. authoritative simulation depth and cross-domain coupling;
2. Explanation/analytics kept current with accepted simulation capabilities;
3. bounded observer read models and protocol support required for inspection;
4. coherent UI milestone batches after read models stabilize.

Detailed work advances capabilities through documented, contracted, executable, coupled,
observable, and validated maturity levels. Foundation `Completed` labels are historical scope
statements, not claims that a whole domain is mature.

Immediate program priorities are durable physical-state coupling, production historical bootstrap
without fixtures, domain-valid recovery and phenomenon metrics, and causal/domain inspection. Four
bounded vertical slices are complete: the actor/material/mana loop (`plans/actor-material-mana-loop.md`),
the local mana-material-surface coupling (`plans/local-mana-material-surface-coupling.md`), the
bounded conserved thermal storage and same-chart transfer slice (`plans/conserved-thermal-energy-carrier.md`),
terrain carrier participation (`plans/terrain-carrier-participation.md`), which makes the world
seed reach the running simulation so that two seeds produce two different worlds, and the field
raster projection (`plans/observer-field-raster-map.md`), which gives the observer the per-cell
terrain and mana lattices and the chart a second dimension, so the map draws measured fields instead
of one aggregate per chunk.
The frozen-baseline maturity audit is paused after preserving its completed groundwork; it does not
block accepted vertical-slice work.

A completed performance investigation (`plans/performance-baseline-and-digest-cost.md`) root-caused
`TODO-PERF-001`'s representative-performance gap: two full-rescan digests unconditionally recomputed
every tick, one of them (`history_digest`) unbounded by design over the causal trace store and the
other (`physical_state_digest`) unbounded by omission over an unpruned thermal-receipt log, plus a
`RuntimeConfig` validation gap that admitted actor/sensor/surface-contact combinations the cognition
layer's perception cap cannot execute. All five waves landed: a checked-in statistical harness,
construction-time rejection of unrunnable configurations, an incremental `history_digest` whose value
is bit-identical and asserted so against a retained full-rescan oracle, and CI capture of the
harness's output per commit. Two follow-ups it deliberately did not close are carried forward as
`TODO-PERF-002` (`physical_state_digest`'s thermal-receipt growth, which needs a design decision
rather than an implementation) and `TODO-PERF-003` (regression flagging and a reference-hardware run,
which need a historical series first).

See `docs/architecture/detailed-development-rebaseline.md`; the completed planning record is in
`plans/history/detailed-development-rebaseline.md`.

## Optional LLM surface — terminal gate, unnumbered

Optional LLM wording is removed from the numbered roadmap. It may be considered only after the
simulation's target scope has validated maturity, deterministic Explanation can already explain
the world and experiments, structured source packets are inspectable through the observer/UI, and
persistence, provenance, determinism, and performance gates pass. A future dedicated RFC is
required. LLMs remain optional and non-authoritative under INV-011.
