# Populations

Biological populations are groups of organisms that share ancestry and can interbreed. Ground Truth represents populations as lineages with trait distributions, not as discrete categories.

## Population Representation

```text
PopulationState:
    lineage_id: PopulationLineageId
    geographic_range: Polygon
    population_size: int
    age_structure: AgeDistribution
    sex_ratio: float
    trait_distributions: {TraitId → Distribution}
    genetic_diversity: float
    historical_changes: [PopulationChange]
```

## Biological Variation

Populations exhibit variation:

- **Within-population**: individuals vary around population mean
- **Between-population**: different populations have different trait distributions
- **Continuous**: many traits vary gradually across space
- **Correlated**: traits may covary (e.g., stature and mass)

## Fantasy "Races"

Ground Truth should not generally use socially loaded fantasy race enums.

The engine may represent biological population lineages with distributions such as:

- lifespan tendencies
- fertility
- development timing
- sensory ranges
- morphology
- metabolism
- mana coupling

Agents and societies may construct categories such as:

- elf
- human
- half-elf
- demon

Their boundaries may not match objective biological population structure.

This allows:
- biological continua
- mixed ancestry
- incorrect taxonomies
- social classification conflicts

Observer UI may use familiar glosses when confidence is high. It must expose that these are analytical or local categories.

## Population Dynamics

Populations change through:

- **Births**: reproduction adds individuals
- **Deaths**: mortality removes individuals
- **Migration**: movement changes geographic distribution
- **Selection**: environmental pressure changes trait distributions
- **Drift**: random changes in small populations
- **Mutation**: new genetic variation

## Population and Ecology

Populations interact with ecology:

- **Predation**: populations may prey on or be preyed upon
- **Competition**: populations may compete for resources
- **Symbiosis**: populations may cooperate
- **Niche**: each population occupies an ecological niche

## Population and Society

Societies construct categories from population patterns:

- **Taxonomy**: classification of organisms
- **Husbandry**: management of domesticated populations
- **Hunting**: exploitation of wild populations
- **Conservation**: protection of valued populations

These practices are emergent, not primitive.

## Determinism

Population processes must be deterministic given:

- initial population state
- environmental conditions
- biological parameters
- random stream (for stochastic aspects)

## Performance

Population data may be large. Strategies:

- Aggregate representation for common populations
- Individual tracking for significant organisms
- Statistical genetics for trait distributions
- Spatial batching for geographic processes

## Related Documents

- `architecture.md` — biological system overview
- `heredity.md` — genetic transmission
- `reproduction.md` — birth processes
- `death.md` — mortality
- `demography.md` — population dynamics
- `docs/world/ecology.md` — ecological interactions

## TODO Categories

- `BIO` — biology
- `DEMO` — demography
- `ECO` — ecology
