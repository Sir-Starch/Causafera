# Pathogens

Pathogens are disease-causing biological agents. The engine must not require predefined social disease categories. Ground Truth contains pathogen lineages and physiological state transitions. Agents observe symptom patterns and construct subjective illness concepts.

## Pathogen Representation

```text
PathogenState:
    pathogen_id: PathogenId
    pathogen_type: PathogenType
    virulence: float
    transmission_rate: float
    incubation_period: Time
    infectious_period: Time
    host_range: [PopulationLineageId]
    mutation_rate: float
    environmental_survival: float
```

### Pathogen Types

- **Bacteria**: cellular pathogens
- **Viruses**: subcellular pathogens
- **Fungi**: eukaryotic pathogens
- **Parasites**: multicellular pathogens
- **Prions**: protein pathogens

## Infection Process

### Transmission

Transmission occurs through:

- **Contact**: direct physical contact
- **Airborne**: respiratory droplets
- **Vector**: insect or animal intermediary
- **Waterborne**: contaminated water
- **Foodborne**: contaminated food
- **Fomite**: contaminated objects

Transmission depends on:
- pathogen properties
- host susceptibility
- environmental conditions
- social behavior

### Infection Course

```text
Exposure
    ↓
Incubation (asymptomatic, non-infectious or low-infectious)
    ↓
Prodrome (early symptoms)
    ↓
Clinical illness (symptoms, infectious)
    ↓
Resolution (recovery, death, or chronic state)
```

## Disease Ecology

Disease ecology interacts with:

- **Geography**: climate determines vector ranges; terrain determines water contamination risk
- **Hydrology**: water sources may harbor waterborne pathogens
- **Migration**: movement spreads pathogens between populations
- **Practices**: hygiene, food preparation, burial practices affect transmission
- **Concepts**: understanding of disease affects behavior
- **Medicine**: treatment affects outcomes

## Pathogen Evolution

Pathogens may evolve:

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
- random stream (for stochastic aspects)

## Performance

Pathogen simulation may be expensive for epidemics. Strategies:

- Aggregate representation for population-level epidemics
- Individual simulation for significant infections
- Event-driven transmission
- Spatial batching for environmental transmission

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
