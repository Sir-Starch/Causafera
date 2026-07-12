# Pathogens

Pathogens are disease-causing biological agents. The engine must not require predefined social disease categories. Ground Truth contains pathogen lineages and physiological state transitions. Agents observe symptom patterns and construct subjective illness concepts.

## Implemented Contract Boundary

```text
PathogenLineage:
    id: PathogenId
    parent: PathogenId?
    properties: PathogenProperties
    host_interactions: ordered [HostInteraction]

PathogenProperties:
    minimum_infectious_dose: PathogenQuantity
    shedding_per_tick: PathogenQuantity
    environmental_persistence: FractionPpm
    mutation_propensity: FractionPpm
    incubation: TickDuration
    infectious: TickDuration
```

`ontopolis-biology` implements these immutable Phase 5 extension contracts in `pathogens.rs`. Fractions are bounded integer parts per million and durations are positive simulation ticks. `PathogenLineages` validates a unique, parent-before-child ancestry forest and preserves canonical structure-of-arrays iteration order.

Phase 6 now provides the generic `CausalTraceStore` and proposal/reduce/commit event contracts that future infection systems may use. No infection establishment, host mutation, shedding process, recovery, immunity, or epidemic scheduler has been implemented merely by adding that generic infrastructure.

There is deliberately no authoritative pathogen-type enum. Classifications such as bacteria, virus, fungus, parasite, and prion require evidence and analytical purpose; they are not needed to establish the physical simulation contract.

## Infection Process

### Transmission

`PathogenExposure` records a pathogen lineage, optional source body, target body, positive physical dose, simulation time, and causal trace. It records a transmission opportunity, not an infection outcome.

There is deliberately no transmission-route enum. Material contact, suspended particles, fluids, food, environmental surfaces, and intermediary organisms must create dose through their physical domain processes. A label such as "airborne" cannot bypass that causal path.

### Infection Course

The implemented contract supplies incubation and infectious durations but does not implement infection stages. Exposure, establishment, physiological effects, recovery, chronic state, and death will require scheduler-controlled proposal/reduce/commit processes with causal provenance.

### Host Interaction

Each lineage may contain a canonically ordered profile keyed by objective `PopulationLineageId`. A profile stores bounded susceptibility, replication compatibility, and damage response. These values are physical model inputs. Host lineage IDs are not social population categories and are never directly available to agent cognition.

## Disease Ecology

Disease ecology interacts with:

- **Geography**: climate determines vector ranges; terrain determines water contamination risk
- **Hydrology**: water sources may harbor waterborne pathogens
- **Migration**: movement spreads pathogens between populations
- **Practices**: hygiene, food preparation, burial practices affect transmission
- **Concepts**: understanding of disease affects behavior
- **Medicine**: treatment affects outcomes

## Pathogen Evolution

Future pathogen processes may evolve through:

- **Mutation**: genetic change creating new variants
- **Selection**: pressure favoring certain traits
- **Adaptation**: improved host exploitation
- **Antigenic drift**: gradual change in surface proteins
- **Antigenic shift**: major change creating novel pathogen

## Social Disease Concepts

Different societies may classify the same pathogen differently:

- **Symptom-based**: "fever disease", "coughing sickness"
- **Cause-based**: "bad air", "divine punishment", "imbalance"
- **Location-based**: "South Canal fever", "winter illness"
- **Social-based**: "traveler's disease", "poor people's illness"

These classifications are subjective. Ground Truth stores pathogen lineages and physiological effects.

## Determinism

Pathogen processes must be deterministic given:

- pathogen properties
- host state
- environmental conditions
- transmission opportunities
- scheduler-provided random stream (for future stochastic aspects)

Implemented construction consumes no randomness or floating point. Lineage ancestry and host profiles have canonical order. Future transmission and evolution outcomes must use deterministic stream keys and stable operation ordinals.

## Performance

The implemented registry uses contiguous lineage fields and cold boxed host profiles. No performance or epidemic-scale claim is made. Candidate strategies requiring benchmarks include:

- Aggregate representation for population-level epidemics
- Individual simulation for significant infections
- Event-driven transmission
- Spatial batching for environmental transmission

## Current Non-Goals

- infection or immune-state mutation
- pathogen generation, mutation algorithms, or epidemic scheduling
- pathogen-type, transmission-route, disease, or symptom enums
- molecular and cellular biology
- observer protocol and persistence formats

## Related Documents

- `architecture.md` — biological system overview
- `physiology.md` — host physiological response
- `populations.md` — population-level disease dynamics
- `demography.md` — mortality and morbidity
- `docs/world/ecology.md` — ecological disease reservoirs

## TODO Categories

- `PATH` — pathogens
- `BIO` — biology
- `DEMO` — demography

## RFCs

- `RFC-BIO-002: Minimal Pathogen Contracts` — Accepted and implemented
