# Heredity

Heredity is the transmission of biological traits from parents to offspring. It creates variation within populations and enables adaptation over generations.

## Hereditary Representation

Heredity is represented as a genetic or trait transmission system:

```text
HeredityState:
    parentage: (BiologicalEntityId, BiologicalEntityId)
    inherited_traits: {TraitId → InheritedValue}
    genetic_diversity: float
    mutation_accumulation: float
    lineage_history: [LineageEvent]
```

### Trait Inheritance

```text
TraitId:
    base_value: float
    heritability: float
    environmental_sensitivity: float
    dominance: DominanceType
```

Trait inheritance combines:
- **Genetic contribution**: from parents
- **Environmental contribution**: from developmental conditions
- **Mutation**: random or stress-induced changes

## Trait Types

Traits include:

- **Morphological**: stature, proportions, segment count
- **Physiological**: metabolic rate, sensory range, immune response
- **Developmental**: growth rate, maturation timing, lifespan
- **Behavioral**: temperament, learning rate, social tendency
- **Mana-related**: mana sensitivity, coupling tendency

## Variation and Adaptation

Heredity creates variation:

- **Within populations**: individuals vary around population mean
- **Between populations**: different lineages have different trait distributions
- **Over time**: populations shift through selection and drift

Adaptation occurs when:
- environmental pressure favors certain traits
- those traits become more common
- population mean shifts

## Heredity and Society

Societies may construct categories based on hereditary traits:

- **Family**: shared ancestry
- **Lineage**: extended kinship
- **Caste**: hereditary social position
- **Race**: perceived hereditary group

These categories are emergent. Ground Truth stores biological lineages. Societies construct social categories from observed patterns.

## Heredity and Other Domains

Heredity interacts with:

- **Development**: genetic parameters determine developmental trajectory
- **Physiology**: inherited traits determine physiological capacity
- **Morphology**: inherited parameters determine body structure
- **Populations**: heredity creates population structure
- **Ecology**: heredity enables adaptation to ecological conditions
- **Mana**: hereditary mana traits may run in families

## Determinism

Heredity must be deterministic given:

- parent traits
- inheritance rules
- mutation parameters
- environmental conditions

## Performance

Heredity data may be large for populations. Strategies:

- Represent trait distributions rather than individual genomes where possible
- Use statistical genetics for population-level processes
- Track detailed heredity only for significant lineages

## Related Documents

- `architecture.md` — biological system overview
- `morphology.md` — inherited morphology
- `physiology.md` — inherited physiology
- `development.md` — genetic influences on development
- `reproduction.md` — trait transmission mechanism
- `populations.md` — population-level genetics

## TODO Categories

- `BIO` — biology
- `DEMO` — demography
