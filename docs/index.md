# Causafera Documentation Index

## Specification Provenance

This repository was initialized from a project initialization specification. That specification remains the historical baseline.

Post-initialization architectural discoveries, refinements, and course corrections are recorded through:

- **ADRs** — individual architecture decisions in `docs/adr/`;
- **RFCs** — design investigations for complex subsystems in `docs/rfc/`;
- **Architecture rebaseline documents** — mid-course corrections in `docs/architecture/`;
- **Subsystem documentation** — detailed domain documentation throughout `docs/`;
- **Roadmap revisions** — phase resequencing and scope updates in `docs/roadmap/`.

Later documents supersede conflicting initialization-spec sections **only when they explicitly say so**.

## Current Documentation Guide

- **Project overview** — [README.md](../README.md), the [project thesis](vision/project-thesis.md), and [what distinguishes Causafera](vision/uniqueness.md).
- **Architecture and concepts** — the [invariants](architecture/invariants.md), [Detailed Development rebaseline](architecture/detailed-development-rebaseline.md), [domain coverage matrix](ontology/domain-coverage-matrix.md), and the subsystem sections below.
- **Governance and licensing** — [GOVERNANCE.md](../GOVERNANCE.md) is the authoritative statement of decision-making authority; the [CLA](../CLA.md) governs accepted contributions, and [`docs/legal/`](legal/cla-service-setup.md) holds the maintainer-side setup material.
- **Development and contribution** — [CONTRIBUTING.md](../CONTRIBUTING.md), [development notes](development/), and [CHANGELOG.md](../CHANGELOG.md).
- **Agent and plan guidance** — [AGENTS.md](../AGENTS.md) is the canonical guidance for every coding agent; [PLANS.md](../PLANS.md) defines ExecPlan authority, structure, and the current plan list.
- **Historical records** — completed ExecPlans and public-source-readiness provenance are retained in [historical plans and records](../plans/history/README.md); they are not current implementation guidance.
- **Audit and maturity material** — [the paused maturity audit](../plans/detailed-development-maturity-audit.md) is historical diagnostic groundwork, not a public project-status claim or an implementation prerequisite.

## Vision

- `docs/vision/project-thesis.md` - Central project thesis and mana semantics
- `docs/vision/core-loop.md` - Fundamental causal loop
- `docs/vision/isekai-targets.md` - Target emergent phenomena
- `docs/vision/uniqueness.md` - What distinguishes Causafera

## Ontology

- `docs/ontology/world-ontology.md` - World ontology overview
- `docs/ontology/domain-coverage-matrix.md` - Domain coverage analysis
- `docs/ontology/causal-carriers.md` - Causal carrier documentation
- `docs/ontology/lifecycle-audit.md` - Entity lifecycle audit framework
- `docs/ontology/cross-domain-interactions.md` - Cross-domain interaction matrix
- `docs/ontology/primitive-vs-emergent.md` - Primitive vs emergent distinction
- `docs/ontology/unresolved-assumptions.md` - Unresolved assumptions

## Architecture

- `docs/architecture/invariants.md` - Hard invariants
- `docs/architecture/performance.md` - Performance philosophy
- `docs/architecture/determinism.md` - Determinism requirements
- `docs/architecture/data-oriented.md` - Data-oriented storage
- `docs/architecture/observer.md` - Observer architecture
- `docs/architecture/protocol.md` - Observer protocol
- `docs/architecture/provenance.md` - Ground Truth events and causal provenance
- `docs/architecture/external-consciousness-archive.md` - Proposed external archive and restoration boundaries
- `docs/architecture/operator-physical-interventions.md` - Proposed constrained physical and mana intervention boundaries
- `docs/architecture/cognition-rebaseline.md` - Cognition rebaseline: subjective scene and cognitive continuity
- `docs/architecture/detailed-development-rebaseline.md` - Post-Phase-26 detailed development priorities, maturity gates, and terminal LLM policy

## World

- `docs/world/geography-philosophy.md` - Geography philosophy
- `docs/world/spatial-hierarchy.md` - Spatial hierarchy
- `docs/world/coordinates.md` - Coordinate systems
- `docs/world/terrain.md` - Terrain
- `docs/world/geology.md` - Geology
- `docs/world/hydrology.md` - Hydrology
- `docs/world/climate.md` - Climate
- `docs/world/ecology.md` - Ecology
- `docs/world/settlements.md` - Settlements
- `docs/world/mana-topology.md` - Mana topology
- `docs/world/world-generation-provenance.md` - World generation provenance
- `docs/world/historical-bootstrap.md` - Historical bootstrap

## Biology

- `docs/biology/architecture.md` - Biological architecture
- `docs/biology/morphology.md` - Morphology
- `docs/biology/physiology.md` - Physiology
- `docs/biology/development.md` - Development
- `docs/biology/heredity.md` - Heredity
- `docs/biology/reproduction.md` - Reproduction
- `docs/biology/aging.md` - Aging
- `docs/biology/death.md` - Death
- `docs/biology/pathogens.md` - Pathogens
- `docs/biology/populations.md` - Populations
- `docs/biology/demography.md` - Demography
- `docs/rfc/RFC-BIO-003.md` - External biological mana coupling, acquired retention, and emergent practitioners

## Cognition

- `docs/cognition/attention.md` - Attention
- `docs/cognition/memory.md` - Memory
- `docs/cognition/salience.md` - Salience
- `docs/cognition/prediction.md` - Prediction
- `docs/cognition/belief-inertia.md` - Belief inertia
- `docs/cognition/goals.md` - Goals
- `docs/cognition/habits.md` - Habits
- `docs/cognition/trust.md` - Trust
- `docs/cognition/strategic-communication.md` - Strategic communication

## Language

- `docs/language/architecture.md` - Language architecture
- `docs/language/semantic-layer.md` - Semantic layer
- `docs/language/lexicon.md` - Lexicon
- `docs/language/phonology.md` - Phonology
- `docs/language/morphology.md` - Morphology
- `docs/language/grammar.md` - Grammar
- `docs/language/communication.md` - Communication
- `docs/language/lexical-innovation.md` - Lexical innovation
- `docs/language/semantic-drift.md` - Semantic drift
- `docs/language/language-change.md` - Language change
- `docs/language/translation.md` - Translation
- `docs/language/writing-systems.md` - Writing systems
- `docs/language/language-bootstrap.md` - Language bootstrap

## Epistemics

- `docs/epistemics/architecture.md` - Epistemic architecture
- `docs/epistemics/knowledge-types.md` - Knowledge types
- `docs/epistemics/measurement.md` - Measurement
- `docs/epistemics/metrology.md` - Metrology
- `docs/epistemics/instruments.md` - Instruments
- `docs/epistemics/experiments.md` - Experiments
- `docs/epistemics/replication.md` - Replication
- `docs/epistemics/science.md` - Science
- `docs/epistemics/writing.md` - Writing
- `docs/epistemics/documents.md` - Documents
- `docs/epistemics/document-lineage.md` - Document lineage

## Practices

- `docs/practices/architecture.md` - Bounded practice programs and execution boundary
- `docs/practices/lineages.md` - Practice ancestry, mutation, and future transmission

## Isekai

- `docs/isekai/architecture.md` - Isekai architecture
- `docs/isekai/transfer-types.md` - Transfer types
- `docs/isekai/foreign-memory.md` - Foreign memory
- `docs/isekai/imported-priors.md` - Imported priors
- `docs/isekai/translation-impact.md` - Translation impact
- `docs/isekai/historical-arrivals.md` - Historical arrivals
- `docs/isekai/causal-contamination.md` - Causal contamination

## Metaphysics

- `docs/metaphysics/identity.md` - Identity
- `docs/metaphysics/death-and-persistence.md` - Death and persistence
- `docs/metaphysics/cross-world-continuity.md` - Cross-world continuity
- `docs/metaphysics/attractors.md` - Attractors
- `docs/metaphysics/gods-and-spirits.md` - Gods and spirits
- `docs/metaphysics/artifacts.md` - Artifacts

## Simulation

- `docs/simulation/perceptual-features.md` - Perceptual features
- `docs/simulation/emergent-concepts.md` - Emergent concepts
- `docs/simulation/technology-and-invention.md` - Technology and invention
- `docs/simulation/maintenance.md` - Maintenance
- `docs/simulation/long-run-experiments.md` - Executable deterministic long-run experiments

## City

- `docs/city/parcels.md` - Parcels
- `docs/city/buildings.md` - Buildings
- `docs/city/streets.md` - Streets
- `docs/city/infrastructure-networks.md` - Infrastructure networks
- `docs/city/maintenance.md` - Maintenance
- `docs/city/urban-growth.md` - Urban growth
- `docs/city/fire.md` - Fire

## Society

- `docs/society/law.md` - Law
- `docs/society/contracts.md` - Contracts
- `docs/society/bureaucracy.md` - Bureaucracy
- `docs/society/records.md` - Records

## Explanation

- `docs/explanation/architecture.md` - Explanation architecture
- `docs/explanation/analytical-ontology.md` - Analytical ontology
- `docs/explanation/classification.md` - Classification
- `docs/explanation/explanation-ir.md` - Explanation IR
- `docs/explanation/confidence.md` - Confidence
- `docs/explanation/causal-summaries.md` - Causal summaries
- `docs/explanation/glossing.md` - Glossing
- `docs/explanation/deterministic-rendering.md` - Deterministic rendering
- `docs/explanation/localization.md` - Localization
- `docs/explanation/optional-llm-surface.md` - Unscheduled terminal-gate policy for optional LLM wording

## Observer

- `docs/observer/architecture.md` - Observer architecture
- `docs/observer/protocol.md` - Protocol
- `docs/observer/snapshots.md` - Snapshots and deltas
- `docs/observer/backpressure.md` - Backpressure

## UI

- `docs/ui/views.md` - User views
- `docs/ui/observer-application.md` - Desktop observer frontend architecture and operation
- `docs/ui/observer-projection-gaps.md` - Observer projections the frontend is waiting on
- `docs/ui/map-lenses.md` - The chart instrument's lens contract and how to extend it
- `docs/ui/map-perspectives.md` - Map perspectives
- `docs/ui/language-inspection.md` - Language inspection
- `docs/media/README.md` - Screenshots of real runtime output, and how to retake them

## Analytics

- `docs/analytics/phenomenon-evaluation.md` - Phenomenon evaluation

## Performance

- `docs/performance/philosophy.md` - Performance philosophy
- `docs/performance/metrics.md` - Metrics
- `docs/performance/benchmarks.md` - Benchmarks

## Governance and Legal

- `GOVERNANCE.md` - Author-led governance, maintainer authority, forks, and why the engine is FOSS
- `CONTRIBUTING.md` - Authoritative contribution policy, flow, AI-agent rules, and validation
- `CLA.md` - Contributor License Agreement, version 1.1 (stable signable text; acceptance status is tracked in `CONTRIBUTING.md`)
- `docs/legal/cla-service-setup.md` - Maintainer checklist for enabling CLA acceptance

## Development

- `docs/development/codebase-memory.md` - Optional codebase knowledge graph tooling
- `docs/development/contributing.md` - Contributing
- `docs/development/changelog.md` - Changelog
- `docs/development/maturity-audit-groundwork.md` - Preserved frozen-audit Todos 1–4 groundwork

## Roadmap

- `docs/roadmap/roadmap.md` - Project roadmap

## RFCs

- `docs/rfc/` - Request for Comments
- `docs/rfc/RFC-COG-001.md` - Subjective Scene and Cognitive Continuity Model
- `docs/rfc/RFC-TRACE-001.md` - Deterministic Ground Truth Event Provenance
- `docs/rfc/RFC-PERCEPT-001.md` - Physical Access, Generic Extraction, and Attention Boundary
- `docs/rfc/RFC-SCENE-001.md` - Bounded Subjective Scene and Cognitive Continuity
- `docs/rfc/RFC-CONCEPT-001.md` - Sparse Subjective Concept Formation
- `docs/rfc/RFC-MANA-001.md` - Minimal Information-Sensitive Field Model
- `docs/rfc/RFC-RES-001.md` - Causal Resolution and State Aggregation
- `docs/rfc/RFC-SOCIAL-001.md` - Distributed Social and Institutional Records
- `docs/rfc/RFC-ECON-001.md` - Traceable Material Economy Foundation
- `docs/rfc/RFC-CITY-001.md` - Physical Urban Infrastructure Foundation
- `docs/rfc/RFC-HIST-001.md` - Causal Historical Bootstrap Orchestration
- `docs/rfc/RFC-ISEKAI-001.md` - Cross-World Transfer and Imported Priors
- `docs/rfc/RFC-META-001.md` - Identity and Post-Biological Pattern Persistence Research
- `docs/rfc/RFC-META-002.md` - Stateful Mana Attractor Research
- `docs/rfc/RFC-GEO-002.md` - Multiscale World Spatial Geometry and Coordinate Model

## ADRs

- `docs/adr/` - Architecture Decision Records

## Glossary

- `docs/glossary.md` - Glossary

## Bibliography

- `docs/bibliography.md` - Bibliography
