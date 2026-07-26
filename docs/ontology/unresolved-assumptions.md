# Unresolved Assumptions

Causafera contains deliberate research hypotheses and explicitly unresolved questions. This document records them so they are not accidentally hardcoded as decided architecture.

## Metaphysics

### Identity and persistence

RFC-META-001 now settles the neutral research boundary: bounded trace-backed observations are evaluated independently under explicit opaque weighted criteria and never produce an authoritative same-person verdict. The physical basis of identity persistence, death, reincarnation, ghosts, and cross-world memory remains unresolved. Do not use a primitive `Soul` object.

See `RFC-META-001: Identity and Post-Biological Pattern Persistence`.

### Gods and spirits

Gods and spirits are target emergent phenomena. The hypothesis is that persistent distributed information and mana structures may form stable stateful attractors. A religious system produces repeated names, symbols, synchronized practices, architecture, calendars, and recurring behavioural structures. These patterns may create a stable mana attractor which eventually exhibits state persistence and responsive behaviour.

RFC-META-002 now supplies bounded read-only field-trajectory stability and perturbation-recovery probes. This remains a research hypothesis: numeric trajectory evidence does not instantiate or prove a god, spirit, artifact, agency, or attractor entity.

See `RFC-META-002: Stateful Mana Attractors`.

### Artifacts

Artifact formation is a research target. The candidate process involves material objects, persistent repeated use, stable physical patterns, local mana coupling, and historical persistence. A bell used at the same time for centuries may develop persistent coupling. A currency token repeatedly exchanged through millions of social transactions may develop unusual effects.

No `EnchantItem` action exists in the engine.

## Spatial Geometry

RFC-GEO-002 settles the dimensional boundary: the default world is a finite closed charted planetary surface; global geography is fixed-point 2.5D; coarse subsurface is layered; local physical frames are full bounded Euclidean 3D; causally relevant subsurface/structures may promote to volumetric 3D. Geometry, containment, jurisdiction, rendering, and causal resolution are separate.

The concrete planetary metric and shape, atlas generation, curvature-aware cross-chart transforms, geodesics, horizon, chart-qualified migration of existing terrain/mana/resolution state, and conservation-safe volumetric promotion/demotion remain unresolved. Bare `WorldCoord`/`ChunkCoord` values are local-chart lattice addresses and must not be treated as a global planetary embedding.

## Isekai Transfer

Cross-world transfer must be a physical or metaphysical process. Possible interpretations include full physical transfer, identity-pattern transfer, partial memory transfer, reincarnation-like binding, informational echo, artifact transfer, and overlapping identity patterns.

RFC-ISEKAI-001 settles only the Phase 22 neutral boundary: opaque mechanism schemas, objective payload/property correspondence, deterministic plans, exact committed receipts, subjective imported priors, and independently evidenced capability. Which mechanism exists and what constitutes identity continuity remain unresolved for Phase 23 research.

Do not select a final metaphysical model during Phase 0.

See `RFC-ISEKAI-001: Cross-World Transfer Model`.

## Mana

RFC-MANA-001 now settles the Phase 17 minimum: a bounded fixed-point scalar field responds to opaque physical fingerprints through recurrence, regular intervals, synchronization, repeated coordinates, magnitude, diffusion, decay, and saturation. Evolution is proposal-only and every committed changed cell requires causal provenance.

The final field physics remain open. Vector state, explicit phase/interference, cross-chart transport, sparse/multi-resolution layouts, acceleration, and empirical parameter selection are deferred. Stateful attractors remain a separate metaphysical research hypothesis.

Four items this list previously deferred are now implemented and are no longer open questions: same-chart cross-chunk exchange, threshold/hysteresis field-to-matter effects, and two concrete carrier adapters — the change-driven material surface and the standing terrain structure. What replaced the question is a narrower one. Provenance is settled: a seam cell could commit a change with no causal ancestry, and `TODO-MANA-005` closed that, so no mana cell change is proposed without a cause. Two narrower questions remain: seam delivery does not apply the field's own saturation ceiling, so a cell fed across a seam in a saturated field can exceed `maximum_intensity` (`TODO-MANA-006`), and both the response constants and the lattice were calibrated against a field that no carrier populated (`TODO-MANA-004`).

See `RFC-MANA-001: Minimal Information-Sensitive Field Model`.

RFC-BIO-003 settles the semantic boundary for organisms: mana remains physical state; no unexplained MP or authoritative magic aptitude exists; contextual biological coupling and causally formed retention are permitted, including congenital reserves produced during gestation or development; ritual/history remains the common route; rare active coupling may emerge but is never guaranteed. Concrete carrier variables, transfer/conversion laws, final field components, prevalence, and evolved biological structures remain unresolved implementation and research questions.

See `RFC-BIO-003: Biological Mana Coupling and Emergent Practitioners`.

## Causal Resolution

RFC-RES-001 now settles the Phase 18 decision-field minimum: bounded fixed-point relevance is reduced from directed, trace-backed signals on opaque weighted channels; deterministic decay, saturation, thresholds, and hysteresis select numeric detail ordinals through proposal/commit transitions. Distance is only one possible adapter input and is not privileged by the reducer.

RFC-SOCIAL-001 settles the Phase 19 storage minimum: objective social carriers are bounded, trace-backed distributed records rather than an organization brain. Role, relation, authority, rule, property, and agreement meanings remain contestable and opaque. Lifecycle proposals, governance, enforcement, shared organizational knowledge, and multi-resolution aggregation remain unresolved.

RFC-ECON-001 and RFC-CITY-001 settle the Phase 20 storage minimum: physical lots, transfers, transformations, performed labour, parcels, buildings, and generic infrastructure topology are bounded and trace-backed. Custody does not prove ownership, and infrastructure schemas do not encode universal road/water/sewage meaning. Committed conservation, lifecycle mutation, markets, allocation, flow physics, maintenance, hazards, growth, and multi-resolution aggregation remain unresolved.

Domain-specific aggregation remains under research. Terrain, biology, populations, language, mana, society, and economy still need explicit conservation, promotion, demotion, and provenance rules. Adapter formulas, evaluation cadence, hierarchical propagation, persistence, and carefully isolated observer-focus inputs are also deferred.

See `RFC-RES-001: Causal Resolution and Aggregation`.

## Language Bootstrap

RFC-HIST-001 settles the Phase 21 orchestration minimum: bounded opaque stages provide time, target chunks, numeric detail, parameter fingerprints, deterministic seed contributions, explicit dependencies, and exact committed receipt ancestry. It does not settle concrete domain synthesis.

The exact mechanism for richer initial languages remains unspecified. Historical bootstrap may use lower-resolution cultural simulation and constrained causal synthesis. The resulting lexicon contains internal IDs and generated forms, not manually authored dictionaries. Phase 21 only provides the adapter slot and causal receipt boundary.

See `RFC-LANG-001: Historical Language Bootstrap`.

## Concept Formation

The sparse subjective concept formation algorithm remains a research area. Concepts must be attention-driven and sparse. Do not continuously cluster all world features for every agent.

See `RFC-CONCEPT-001: Sparse Subjective Concept Formation`.

## Practice Representation

RFC-PRACTICE-001 now settles the Phase 15 structural core: bounded ordered operations, subjective numeric conditions, branches, timing, repetition, tolerances, proposal-only execution, and lineage mutation. Physical material/object bindings, actor roles, locations, synchronization across agents, and transmission fidelity remain unresolved future extensions.

See `RFC-PRACTICE-001: Evolvable Practice Representation`.

## Documented Uncertainty

When a system depends on an unresolved assumption, the code and documentation must explicitly state:

- what assumption is being made
- what RFC or research task addresses it
- what would need to change if the assumption is rejected
- why the current placeholder is sufficient for the current phase

Never silently hardcode a research hypothesis as settled architecture.
