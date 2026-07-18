# Phase 5 Pathogen Contracts ExecPlan

## Goal

Complete `TODO-BIO-002` by defining compact, validated, label-free pathogen lineage, transmission, and host-interaction contracts without implementing infection mutation, epidemic scheduling, physiology, or social disease categories.

## Context

Phase 5 already established immutable biological structure. The backlog also assigns pathogen primitives to Phase 5, but `causafera-biology` has no pathogen module. Phase 6 causal provenance is not yet implemented, so this work can safely establish only immutable authoritative inputs and causally referenced transmission observations. Live infections, pathogen evolution, and host-state mutation remain downstream proposal/reduce/commit systems.

## Relevant invariants

- INV-001, INV-002, and INV-027: pathogen Ground Truth and authoritative IDs are not directly exposed to agent cognition.
- INV-005 and INV-006: disease names and pathogen taxonomy labels are not authoritative simulation meaning.
- INV-009: geography must be able to affect pathogen ecology through later physical transmission opportunities.
- INV-014 and INV-016: live transmission and evolution require causal provenance and phase-controlled mutation.
- INV-017 and INV-018: lineage and host-profile storage is compact and deterministic; no performance claim is made without benchmarks.
- INV-024: host population lineage and socially constructed disease or population categories remain distinct.
- INV-034: pathogen-host Ground Truth cannot become an omniscient subjective body schema.

## Ontology domains affected

- Biology: pathogen lineages and host compatibility become authoritative causal properties.
- Time: incubation and infectious intervals use explicit simulation ticks.
- Population biology: host interaction is keyed by objective population lineage identity, not a social category.

## Causal carriers affected

- Pathogen transmission: an exposure carries pathogen material from an optional source body to a target body.
- Ecological pressure and migration may later generate exposure records.
- Physiological damage, immune response, symptoms, medicine, and social disease concepts remain downstream.

## Relevant documents

- `docs/biology/architecture.md`
- `docs/biology/pathogens.md`
- `docs/biology/physiology.md`
- `docs/biology/populations.md`
- `docs/ontology/domain-coverage-matrix.md`
- `docs/ontology/primitive-vs-emergent.md`
- `docs/ontology/causal-carriers.md`
- `docs/ontology/cross-domain-interactions.md`
- `docs/architecture/invariants.md`
- `docs/architecture/data-oriented.md`
- `docs/architecture/determinism.md`
- `docs/adr/ADR-001.md`
- `docs/adr/ADR-002.md`
- `docs/adr/ADR-003.md`
- `docs/adr/ADR-004.md`
- `docs/rfc/RFC-BIO-001.md`
- `docs/rfc/RFC-BIO-002.md`

## Current state

`causafera-types` supplies typed `PathogenId`, `PopulationLineageId`, `BodyId`, `TraceId`, and `SimulationTime` values. `causafera-biology` has structural morphology plus placeholder physiology and population types, but no pathogen representation. The pathogen document currently sketches semantic pathogen and route enums that conflict with the repository rule against convenience domain taxonomies.

## Proposed architecture

Represent bounded fractional traits as integer parts per million and durations as positive simulation-tick counts. A `PathogenLineage` contains a typed identity, optional parent lineage, property-only transmission and progression parameters, and a canonically ordered host-interaction profile. Host profiles describe susceptibility, replication compatibility, and damage response for objective population lineages without naming diseases or symptoms.

Represent a physical transmission opportunity as `PathogenExposure`: pathogen lineage, optional source body, target body, positive material dose, simulation time, and causal trace reference. Do not encode route enums such as airborne, contact, or waterborne; later physical systems produce exposure dose through material contact, motion, fluids, ingestion, vectors, or environmental reservoirs.

A `PathogenLineages` structure-of-arrays registry validates unique lineage identities, parent-before-child canonical order, and lineage/property alignment. Its deterministic order is the basis for future batch evolution and epidemic processing.

## Primitive vs emergent review

Pathogen lineage identity, ancestry, exposure dose, timing, bounded physical rates, environmental persistence, and host compatibility are Ground Truth properties. Bacteria, virus, parasite, disease, symptom, plague, immunity category, and route names are classifications or models and are not stored as authoritative enums.

## Non-goals

- Infection state mutation, recovery, chronic illness, death, or immune simulation.
- Pathogen generation, mutation algorithms, natural selection, or epidemic scheduling.
- Named pathogen types, transmission-route enums, diseases, symptoms, or demo outbreaks.
- Physiology, molecular biology, cell simulation, medicine, observer protocol, or narrative explanation.
- Stable persistence formats or GPU acceleration.

## Implementation stages

1. Document and accept the minimal pathogen boundary in RFC-BIO-002.
2. Implement fixed-point rates, durations, lineage properties, validated host profiles, exposure records, and a canonical lineage registry.
3. Test numeric boundaries, profile ordering, ancestry validation, exposure validation, accessors, and identical-input determinism.
4. Update biology and ontology documentation, TODO status, roadmap, changelog, and completed-plan registration.

## Verification

- `cargo fmt --all -- --check`
- `cargo test -p causafera-biology --all-targets`
- `cargo test --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `git diff --check`

## Benchmark plan

No throughput, memory, or epidemic-scale claim is introduced. Before optimizing, benchmark lineage construction, host-profile lookup, sequential registry traversal, bytes per lineage, and large exposure-batch validation using representative synthetic property records rather than fake simulated history.

## Determinism impact

Construction consumes no randomness. All fractions and durations use integers. Host profiles and lineage parents must use explicit canonical order; lookups use stable linear scans. Future stochastic transmission and mutation must consume scheduler-provided deterministic streams keyed beyond a shared global RNG.

## Memory impact

Lineage hot fields are stored in parallel vectors. Host interaction profiles are per-lineage variable-length cold data and remain in deterministic boxed slices. Exposure is a fixed-size value. No memory-efficiency claim is made until measured.

## Observer impact

None. A future observer read model may expose lineage and exposure analytics with confidence and causal trace references. It may apply human classifications only outside authoritative state.

## Explanation impact

None. Exposure records carry `TraceId` so a later Explanation Engine can inspect causal ancestry. It cannot create exposures or mutate host state.

## Persistence impact

No stable snapshot encoding is defined. Integer units, typed IDs, and canonical order prepare the types for a future versioned format without declaring Rust layout to be that format.

## Cross-domain effects

Geography, hydrology, ecology, migration, practices, and material contact may later determine physical exposure dose. Physiology and immune state may consume host-interaction parameters. Cognition receives only physically accessible symptoms and observations, never lineage identity or pathogen Ground Truth.

## Risks

- Semantic pathogen taxonomies could leak into Ground Truth; the contract uses only typed identity and measurable properties.
- A route enum could bypass physical causality; exposures record resulting material dose and trace rather than a named route.
- Live infection mutation before provenance exists would violate INV-014 and INV-016; this phase exposes immutable contracts only.
- Floating-point rates could weaken replay; rates use bounded parts-per-million integers.
- Social categories could be confused with objective host lineages; APIs accept only `PopulationLineageId` and documentation preserves the distinction.

## Documentation changes

Add RFC-BIO-002 and update pathogen architecture, primitive inventory, biology coverage, roadmap, changelog, and plan registration.

## TODO changes

Mark `TODO-BIO-002` completed only after focused and workspace verification pass.

## Decision log

- 2026-07-12: Scope completion to the only pending `BIO` backlog item rather than inventing roadmap phases for future biology sketches.
- 2026-07-12: Use property-only pathogen lineages and omit named pathogen and transmission-route enums.
- 2026-07-12: Represent exposure as physical dose with time and causal trace, not as an immediate infection mutation.
- 2026-07-12: Defer live infection and evolution processes until phase-controlled causal provenance exists.

## Progress

- [x] Required vision, ontology, architecture, biology, ADR, RFC, roadmap, and TODO context reviewed.
- [x] RFC-BIO-002 accepted and pathogen contracts implemented.
- [x] Focused and workspace verification passes.
- [x] Documentation, TODO, roadmap, changelog, and plan registration updated.
