# Post-Phase-24 Simulation Depth

**Status:** Draft

## Goal

Turn the Phase 24 executable causal field laboratory into the first genuinely coupled Ontopolis world simulation by completing eight ordered milestones:

```text
separate state/history identity
→ make experiments interpretable
→ retain temporal physical patterns
→ close mana/physics feedback
→ replace fixture samples with real carriers
→ expand across space and causal resolution
→ add physically grounded biological actors
→ synthesize causal history and run population experiments
```

The result must support deterministic, provenance-complete long runs in which geography, physical patterns, mana, resolution, bodies, perception, subjective cognition, and action participate in one closed causal loop. It must not claim social or metaphysical emergence merely because the loop executes.

## Context

Phase 24 proves that the scheduler can execute a fallible physical → mana → resolution chain for 1,000 ticks with exact replay. The current experiment also exposes the next architectural limits:

- one digest mixes current physical state with the entire causal history;
- checkpoint interpretation is mostly CLI prose and the recovery comparison is not matched to a pre-intervention baseline/control;
- mana sees recurrence inside the current batch but retains no bounded cross-tick physical-pattern history;
- mana has no physical output, so the causal graph is open-ended rather than a feedback loop;
- runtime samples are a fixed three-cell fixture rather than emissions derived from real geometry, motion, material, acoustics, glyphs, or practices;
- one local chunk saturates causal resolution without exercising cross-chunk relevance or domain promotion;
- implemented biology, perception, subjective scene, cognition, language, practice, society, economy, and history contracts are not integrated into the runtime;
- historical bootstrap orchestrates opaque adapters but has no concrete domain synthesis producing a population capable of long-run change.

Phase 25 remains necessary, but its first role is measurement and causal interpretation rather than UI decoration.

## Relevant invariants

INV-001 through INV-007, INV-009 through INV-019, INV-021 through INV-037.

Especially important:

- agents receive only physically accessible, lossy information;
- explanation and observer systems are read-only;
- mana responds only to physical/informational structure;
- mutation remains phase-controlled proposal → reduce → commit;
- all significant transitions retain provenance;
- geometry, containment, jurisdiction, rendering, and resolution remain separate;
- emergence and scale claims require reproducible evidence.

## Ontology domains affected

Determinism, provenance, analytics, explanation, experiment design, physical patterns, mana, matter, geometry, geography, causal resolution, biology, perception, subjective cognition, action, practices, language carriers, historical bootstrap, populations, and persistence boundaries.

## Causal carriers affected

- canonical physical-state and history fingerprints;
- typed analytical claims, confidence, and supporting traces;
- bounded temporal recurrence/periodicity windows;
- mana-to-physical effect proposals;
- geometric, motion, material, acoustic, glyph, and practice-produced samples;
- chart-qualified cross-chunk field exchange and relevance signals;
- sensory signals, subjective cues, action proposals, and physical outcomes;
- aggregate population flows, births/deaths, transmission, material flows, and historical stage receipts.

## Relevant documents

- `docs/vision/project-thesis.md`
- `docs/vision/core-loop.md`
- `docs/architecture/invariants.md`
- `docs/architecture/determinism.md`
- `docs/architecture/provenance.md`
- `docs/architecture/cognition-rebaseline.md`
- `docs/ontology/domain-coverage-matrix.md`
- `docs/ontology/causal-carriers.md`
- `docs/ontology/primitive-vs-emergent.md`
- `docs/simulation/long-run-experiments.md`
- `docs/analytics/phenomenon-evaluation.md`
- `docs/world/coordinates.md`
- `docs/world/mana-topology.md`
- `docs/world/historical-bootstrap.md`
- `docs/biology/architecture.md`
- `docs/simulation/perceptual-features.md`
- `docs/rfc/RFC-TRACE-001.md`
- `docs/rfc/RFC-MANA-001.md`
- `docs/rfc/RFC-RES-001.md`
- `docs/rfc/RFC-GEO-002.md`
- `docs/rfc/RFC-COG-001.md`
- `docs/rfc/RFC-SCENE-001.md`
- `docs/rfc/RFC-HIST-001.md`
- `docs/rfc/RFC-EXPLAIN-001.md`

## Current state

`ontopolis-runtime` owns a centralized synchronized state and registers physical, mana, and resolution systems. A fixed physical source emits three same-fingerprint samples in adjacent cells every tick. The mana field reaches a driven fixed point by approximately tick 256. Temporary source suppression causes decay and later return to the same field state after forcing resumes.

`RuntimeSnapshot::canonical_state` hashes current state and the complete trace store together. The experiment therefore detects different histories but cannot by itself distinguish current-state divergence from provenance divergence.

`AttractorProbe` receives bounded checkpoints, but its recovery measurement does not yet compare matched control, pre-intervention baseline, perturbation state, and recovered state as four separate quantities.

The remaining domain crates mostly expose validated storage and proposal contracts. They are not evidence that a populated world is already simulated.

## Proposed architecture

### Runtime state boundaries

Split runtime identity into independently versioned canonical projections:

- `PhysicalStateDigest`: current authoritative domain state only;
- `HistoryDigest`: committed event/effect/cause graph only;
- `ExperimentDigest`: plan, intervention, checkpoint schema, and both digests;
- optional domain digests for diagnosis, never as competing authorities.

### Analytical boundary

Accept a minimal RFC-EXPLAIN-001 representation containing typed claim schema IDs, numeric values/ranges, confidence, evidence trace references, comparison context, and explicit unsupported/unknown states. Analytical labels and localized text stay outside authoritative state.

### Physical-pattern memory

Introduce a bounded canonical temporal index owned by the physical carrier/mana adapter boundary. It stores only fingerprint, chart-qualified region, tick, position summary, magnitude summary, source ordinal, and trace ancestry. It does not store ritual, word, belief, practice meaning, or observer classification.

### Closed mana loop

Mana evolution remains a field proposal. A separate adapter converts field gradients/state changes into bounded opaque physical-effect proposals. Physics validates target/property compatibility and commits effects before their consequences can generate future physical samples. Mana never writes matter directly.

### Carrier adapters

Real domain state produces `PhysicalPatternSample` values through registered adapters. Each adapter canonicalizes measurable carrier structure and retains exact input traces. Practice/document identities may help locate physical state in Ground Truth bookkeeping, but their meanings never enter mana.

### Spatial and resolution integration

Migrate global field/relevance addressing to `ChartChunkCoord`. Cross-chunk exchange uses explicit chart adjacency/transform contracts. Resolution transitions invoke domain-specific promotion/demotion adapters only after conservation and provenance validation.

### Actor loop

The minimal actor loop is:

```text
objective body + local geometry
→ physically accessible signals
→ generic feature extraction
→ identity-free subjective cues
→ subjective scene / active cognition
→ bounded action proposal
→ physical validation and commit
→ new signals and pattern carriers
```

Production actors are created only by causal bootstrap/lifecycle adapters. Unit tests may use explicitly marked fixtures; runtime must not invent demo residents.

### Historical and population integration

Concrete historical stages operate first on conserved aggregates and later promote causally relevant actors/places. Named settlements, conflicts, discoveries, professions, religions, and technologies are never stage enums. Long-run experiments compare matched seeds, controlled parameter/intervention changes, physical state, history, activity, and provenance metrics.

## Primitive vs emergent review

Primitive:

- canonical state/history schemas and fingerprints;
- physical field values, positions, material properties, motion, time, and traces;
- bounded numeric temporal pattern summaries;
- opaque effect/carrier/relevance schemas;
- objective body state and validated physical actions;
- sensory accessibility and generic features;
- subjective cues/scenes/concepts stored only inside agents;
- conserved aggregate quantities and explicit lifecycle events.

Not primitive:

- ritual, spell, enchantment, sacredness, god, spirit, artifact, soul;
- profession, class, skill, technology level, religion, ethnicity, city character;
- authoritative word meaning or correct social category;
- emergence verdict, narrative significance, or human-facing explanation label.

## Non-goals

- Rich observer UI or narrative realization.
- LLM access to raw state or authoritative mutation.
- Semantic event tables, technology trees, spell lists, jobs, or fake history.
- Full physiology, ecology, hydrology, climate, economy, governance, or language grammar in one batch.
- Planet-wide millimetre voxels or immediate cross-chart planetary simulation.
- CUDA implementation before a profiled CPU reference workload.
- Claims of gods, artifacts, societies, or technologies emerging merely because tests pass.

## Implementation stages

### 1. Separate physical-state identity from causal-history identity

Outcome:

- introduce versioned `PhysicalStateDigest`, `HistoryDigest`, and `ExperimentDigest` types;
- define canonical field ordering and schema-version inputs;
- make snapshots expose physical and history digests independently;
- retain a combined diagnostic digest only as a derived convenience;
- update replay tests to compare both digests;
- update counterfactual tests to distinguish final-state convergence, transient-state divergence, and history divergence.

Acceptance gate:

- control and recovered intervention may have equal physical digest and unequal history digest;
- same seed/plan yields equal physical and history digests at every checkpoint;
- locale, wall time, CLI names, and explanation labels cannot affect either digest;
- digest coverage has mutation tests or explicit field-coverage tests preventing silent omission of new authoritative fields.

Likely code:

- `ontopolis-runtime` snapshots/digest registry;
- `ontopolis-core` canonical hashing/version contracts if reuse is needed;
- `ontopolis-lab` replay/counterfactual assertions.

### 2. Make checkpoints and interventions causally interpretable

Outcome:

- accept the minimal deterministic core of RFC-EXPLAIN-001;
- represent typed numeric claims, evidence traces, confidence, comparison cohort, and unsupported states;
- expose checkpoint series without localization inside the authoritative crates;
- correct recovery analysis to retain pre-intervention baseline, perturbation minimum/maximum, matched control at the same ticks, final recovery distance, and time-to-recovery;
- implement initial phenomenon metrics: reconstructability, causal depth, temporal span, and counterfactual state distance;
- keep analytical classifications read-only.

Acceptance gate:

- the 1,000-tick experiment can deterministically explain field growth, fixed point, suppression, decay, and recovery with supporting traces;
- an explanation distinguishes “driven equilibrium” from “autonomous persistence” and can return “insufficient evidence”;
- removing or changing evidence lowers confidence or invalidates the claim rather than inventing support;
- observer locale changes rendered text only, never IR or simulation digests.

Likely code:

- `ontopolis-explanation` IR and causal query layer;
- `ontopolis-analytics` deterministic metrics;
- `ontopolis-lab` matched checkpoint analysis;
- CLI uses the read-only IR renderer.

### 3. Add bounded cross-tick physical-pattern history

Outcome:

- define a capped canonical `PhysicalPatternHistory` or equivalent temporal index;
- support recurrence across ticks, equal/non-equal interval evidence, bounded periodicity, synchronization, spatial recurrence, decay/eviction, and trace ancestry;
- separate historical pattern evidence from mana field persistence;
- ensure batch partitioning does not change results;
- add experiment plans contrasting synchronous-only, spatial-only, temporally periodic, irregular, and interrupted inputs without semantic type enums.

Acceptance gate:

- three samples in one tick and one sample repeated across three ticks produce distinguishable structural evidence;
- identical logical samples split into different scheduler batches produce identical results;
- old evidence expires or aggregates under a documented bounded policy;
- no unbounded per-pattern history or trace-vector growth;
- a periodic source and an equal-magnitude irregular source measurably diverge for structural reasons.

Likely code:

- new RFC amendment or RFC for temporal pattern retention;
- `ontopolis-domains::mana` input analysis;
- `ontopolis-runtime` carrier staging;
- targeted Criterion benchmarks.

### 4. Close the mana → physics feedback loop

Outcome:

- define opaque `ManaEffectSchemaId` and bounded `ManaPhysicalEffectProposal` contracts;
- derive proposals only from numeric field state/gradients/history and registered physical couplings;
- validate targets, bounds, conservation/energy accounting where applicable, and causal ancestry in the physics phase;
- commit effects as Ground Truth events before changed matter/motion can emit later samples;
- include threshold/hysteresis or response dynamics sufficient to prevent zero-cost oscillation;
- support negative/control cases where mana has no registered coupling.

Acceptance gate:

- a field can cause at least one generic physical property transition through proposal/commit, never direct mutation;
- the resulting physical change can alter a future physical-pattern sample, forming a traced closed loop;
- beliefs, concepts, words, practice meanings, and explanation labels cannot select effects;
- disabling the registered coupling removes the physical effect while leaving field evolution deterministic;
- runaway feedback is bounded and tested.

Likely code:

- new mana-effects RFC;
- `ontopolis-domains` effect proposals;
- `ontopolis-runtime` physics reducer/commit integration;
- provenance target-schema registry.

### 5. Replace fixture samples with real physical carrier adapters

Outcome:

- create a carrier adapter registry with opaque schema/revision IDs;
- implement initial adapters for geometry recurrence, motion trajectories, and material structure;
- then add acoustic waveform summaries, glyph geometry, and practice-produced motion/timing as their physical prerequisites become available;
- require every sample fingerprint to derive from canonical physical representation plus explicit metric scale;
- remove the production three-cell emitter; preserve it only as a named test fixture outside normal runtime construction.

Acceptance gate:

- moving or rotating geometry changes samples according to physical geometry rather than a label;
- structurally identical carriers from different semantic contexts can produce identical fingerprints;
- semantically identical practices with different timing/geometry may produce different fingerprints;
- adapter input traces reconstruct the source geometry/motion/material transition;
- no English string, `ConceptId`, belief, document genre, or practice meaning enters a fingerprint.

Likely code:

- carrier registry near runtime/domain boundary;
- geometry/motion/material adapters in appropriate domain crates;
- acoustic adapter only after physical propagation contract acceptance;
- practice emissions mapped to physical actions, not directly to mana.

### 6. Exercise multiple chunks and real causal resolution

Outcome:

- migrate global mana/resolution/runtime addresses to `ChartChunkCoord` or a versioned equivalent;
- implement bounded same-chart cross-chunk mana exchange;
- define explicit cross-chart boundary contract without pretending bare coordinates are global;
- create multiple real relevance producers from field propagation and material/actor carriers;
- implement at least one conservation-safe domain promotion/demotion adapter;
- make resolution cadence and hysteresis explicit.

Acceptance gate:

- a causally strong distant chunk can reach higher detail than a weak nearby chunk;
- physical distance affects only adapters that explicitly use it;
- promotion/demotion preserves configured conserved quantities and provenance;
- cross-chunk field exchange is bit-identical under different batch/iteration partitioning;
- topology/chart boundaries cannot be crossed by implicit integer adjacency;
- benchmarks report active chunks, transitions, bytes/chunk, and simulated ticks/second without extrapolated scale claims.

Likely code:

- `ontopolis-resolution` domain adapter interfaces;
- `ontopolis-domains::mana` multi-field boundary exchange;
- `ontopolis-geography` chart-qualified terrain migration;
- runtime active-set and chunk registry.

### 7. Integrate the minimal biological actor causal loop

Outcome:

- define the smallest objective body state needed for pose, motion, energy/resource constraint, and sensor placement;
- attach physical sensors through apertures rather than Ground Truth reads;
- run acquisition → extraction → cue bridge → subjective scene → active cognition;
- add a bounded opaque action-selection/proposal contract using subjective state only;
- validate action feasibility against objective body/material/geometry state outside cognition;
- commit action outcomes and feed their physical signals back into later perception/mana;
- keep autobiographical memory, active context, body schema, and objective body state separated.

Acceptance gate:

- an actor can perceive an accessible physical change, construct a subjective scene, propose an action, and produce a committed physical outcome;
- occluded/out-of-range/inaccessible state cannot affect cognition;
- cognition contains no `EntityId`, `PlaceId`, `BodySegmentId`, `SpatialChartId`, `LocalFrameId`, or `TraceId` as subjective meaning;
- an incorrect subjective identity/prediction can lead to a physically valid but mistaken action;
- same seed and inputs replay exactly;
- production runtime creates no demo resident outside a causal bootstrap/lifecycle path.

Likely code:

- biology runtime state and minimal physiology RFC;
- perception geometry/access adapters;
- cognition action-proposal boundary;
- runtime phase integration and actor active sets.

### 8. Implement concrete causal historical adapters and population long runs

Outcome:

- define conserved low-resolution population and material/geographic aggregates;
- implement concrete historical adapters for physical geography initialization, population lifecycle flow, actor promotion, practice/document transmission, and material activity without named historical outcomes;
- create births/deaths/movement/transmission through lower-level proposals and traces;
- connect historical stage receipts to actual committed domain state;
- run matched multi-seed and intervention experiments long enough to observe divergence/convergence across coupled domains;
- apply explanation/analytics metrics without automatically declaring emergence.

Acceptance gate:

- bootstrap produces no fake lore, named city, random war/plague/discovery, or manually authored population history;
- every initialized aggregate and promoted actor has causal ancestry;
- aggregate ↔ detailed transitions conserve documented quantities;
- at least one long-run experiment couples geography, biology, perception/cognition/action, physical patterns, mana, and resolution;
- control/replay equality and intervention differences are reported separately for physical state and history;
- causal depth, domain coupling, path dependence, counterfactual sensitivity, reconstructability, throughput, and memory are measured;
- any claimed phenomenon includes confidence and supporting traces, otherwise the report says evidence is insufficient.

Likely code:

- concrete `HistoricalBootstrapPlan` adapters in world/runtime/domain crates;
- population aggregate/lifecycle storage;
- experiment matrix and reproducible benchmark manifests;
- persistence checkpoint decision before experiments exceed in-memory practical limits.

## Verification

Every stage must pass before the next stage is declared active:

- workspace tests and doctests;
- strict clippy and formatting;
- `git diff --check` and architectural boundary searches;
- deterministic input-order and batch-partition tests;
- same-seed replay at every checkpoint;
- counterfactual tests with explicit expected equality/divergence dimensions;
- provenance parent existence and reconstructability tests;
- locale-independence tests once Explanation IR is active;
- bounded-memory/property tests for histories, active sets, signals, and checkpoints;
- actual CLI/headless experiment invocation;
- refreshed codebase knowledge graph and dependency trace review.

Stage 8 additionally requires a version-controlled experiment manifest recording seed set, parameters, code/schema revisions, warm-up, duration, hardware, wall time, activity counts, memory, state/history digests, and result confidence.

## Benchmark plan

### Baseline preservation

Retain the Phase 24 one-chunk workload as a regression baseline. Record, but do not overinterpret:

- ticks/second;
- causal events and edges/tick;
- mana cell changes/tick;
- resolution transitions/tick;
- checkpoint/digest cost;
- peak RSS where supported.

### Milestone workloads

- Stage 3: pattern-history window size, pattern cardinality, batch partitioning.
- Stage 4: feedback-off, stable bounded feedback, near-limit feedback.
- Stage 5: carrier count and adapter-specific canonicalization cost.
- Stage 6: 1, 16, 256 active chunks; sparse versus dense activity; promotion/demotion.
- Stage 7: 1, 16, 128 active actors with perception/cognition/action enabled.
- Stage 8: low-resolution bootstrap plus promoted focus region over increasing simulated duration.

No million-agent, planetary, or emergence-throughput claim is permitted without a representative reproducible workload.

## Determinism impact

- Digest schemas require stable version IDs and canonical field registration.
- Pattern history must be independent of input order and scheduler batch partitioning.
- Mana effects and carrier adapters use only explicit state, fixed-point arithmetic, stable IDs, time, and deterministic streams.
- Cross-chunk reductions use canonical chart/chunk/channel/source ordering.
- Actor RNG streams remain keyed by world seed, time, phase, system, objective actor bookkeeping ID, and operation ordinal; those IDs never become subjective knowledge.
- Historical adapters derive substreams from explicit stage/process identities.
- Explanation, localization, wall time, hardware counters, and observer focus are excluded from authoritative digests.

## Memory impact

- Pattern histories use fixed windows or deterministic aggregates with hard global/per-region/per-pattern caps.
- Provenance growth is measured; pruning/compaction is forbidden until a persistence and explanation-safe policy is accepted.
- Multi-chunk fields use active sets and bounded boundary buffers.
- Actor hot state is dense/bounded; cold episodic and historical state remains separate.
- Experiment checkpoints store digests and selected numeric projections rather than complete cloned worlds unless persistence explicitly requests a snapshot.
- Stage 8 must establish a practical in-memory ceiling or complete the necessary persistence milestone before longer runs.

## Observer impact

Stages 1–8 remain headless-authoritative. Stage 2 creates read-only analytical projections suitable for future observer transport. No UI panel, locale, map projection, or observer classification may mutate state or affect a digest. Protocol work is required only when external observer delivery becomes part of an acceptance gate.

## Explanation impact

Stage 2 accepts the minimal Explanation IR needed to inspect experiments. Later stages register non-authoritative gloss metadata for opaque schema IDs and add deterministic claims supported by traces. Explanation must expose uncertainty, unsupported claims, comparison context, and gaps. Optional LLM realization remains deferred and receives only validated IR.

## Persistence impact

Stages 1–7 may remain in-memory if bounded experiments fit measured limits. Digest schema versions, chart-qualified identities, event schemas, carrier/effect registries, pattern-history state, actor state, and aggregation state must be designed for eventual exact snapshots.

Before Stage 8 runs exceed the demonstrated in-memory envelope, activate TODO-PERSIST-001 with a separate ExecPlan. Snapshot roundtrip must preserve both physical and history digests exactly.

## Cross-domain effects

- Physical geometry/motion/materials produce mana-readable structure.
- Mana may produce validated generic physical effects, closing feedback.
- Physical outcomes alter accessibility, perception, subjective scenes, actions, and later patterns.
- Actor activity creates material, acoustic, geometric, glyph, and practice carriers.
- Resolution changes representation detail while preserving physical quantities and history.
- Historical population/material processes determine which detailed actors and places exist.
- Explanation and analytics observe all of the above but never participate as causes.

## Risks

- A state digest registry may omit new authoritative fields and create false replay confidence.
- Explanation claims may harden developer labels into hidden semantics.
- Temporal history may grow without bound or double-count recurrence across batch boundaries.
- Mana feedback may create runaway amplification or become a disguised spell-effect table.
- Carrier fingerprints may accidentally hash semantic IDs rather than physical structure.
- Cross-chunk migration may treat local coordinates as global or violate conservation.
- Actor integration may leak Ground Truth identity/geometry into subjective cognition.
- Fixtures may quietly become production residents or fake history.
- Aggregate history may lose causal ancestry or synthesize implausible detail during promotion.
- Provenance volume may dominate runtime before useful long-run duration is reached.
- Completing APIs may be mistaken for demonstrating emergence.

Mitigations are stage gates, hard bounds, opaque schemas, explicit conservation, paired control/replay tests, separate state/history digests, causal evidence requirements, and benchmark-backed scope decisions.

## Documentation changes

On activation and completion of each stage:

- update the relevant RFC or create a narrowly scoped new RFC;
- update `docs/ontology/domain-coverage-matrix.md`;
- update `docs/ontology/causal-carriers.md`;
- update `docs/ontology/primitive-vs-emergent.md`;
- update `docs/ontology/unresolved-assumptions.md`;
- update subsystem documentation and `docs/index.md`;
- update roadmap wording only when the accepted scope changes;
- record measured, reproducible experiment results without emergence claims;
- update changelog and rebaseline notes where architecture boundaries change.

## TODO changes

This draft does not change TODO statuses. When activated, create or refine narrowly scoped TODOs for:

1. canonical state/history digest separation;
2. RFC-EXPLAIN-001 and initial deterministic experiment analytics;
3. bounded temporal physical-pattern retention;
4. mana-to-physical effect proposal/commit;
5. real physical carrier adapters;
6. chart-qualified multi-chunk fields and one domain aggregation adapter;
7. minimal biological actor runtime loop;
8. concrete historical/population adapters and reproducible long-run matrix.

Existing TODO-EXPLAIN-001, TODO-ANALYTICS-001, TODO-PERF-001, TODO-PERSIST-001, and TODO-DET-001 must be reconciled rather than duplicated.

## Decision log

- 2026-07-12: Phase 24 is treated as an executable laboratory baseline, not a populated-world completion claim.
- 2026-07-12: Current-state identity and causal-history identity must be independently comparable.
- 2026-07-12: Phase 25 analytical work is an instrumentation dependency for later emergence experiments.
- 2026-07-12: Temporal recurrence must be measured across bounded physical history, not inferred from accumulated mana alone.
- 2026-07-12: Mana affects matter only through opaque validated physical proposals and commits.
- 2026-07-12: Production mana samples must come from canonical physical carriers, not the three-cell fixture.
- 2026-07-12: Multi-chunk resolution requires chart-qualified geometry and conservation-safe domain adapters.
- 2026-07-12: Actor cognition sees subjective relative cues, never authoritative identity or exact global geometry.
- 2026-07-12: Production populations and recent history originate from causal lifecycle/bootstrap adapters, not demo data or high-level event tables.

## Progress

- [x] 1. Physical-state and causal-history digests separated and verified.
- [x] 2. Deterministic Explanation IR and matched experiment analysis implemented.
- [x] 3. Bounded cross-tick physical-pattern history implemented.
- [x] 4. Mana-to-physics feedback loop closed through proposal/commit.
- [x] 5. Production physical carrier adapters replace fixture emission.
- [x] 6. Chart-qualified multi-chunk fields and causal resolution exercised.
- [x] 7. Minimal biologically grounded actor loop integrated.
- [x] 8. Concrete historical/population adapters and long-run experiment matrix implemented.
