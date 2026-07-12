# Demography

Demography is the study of population structure and change. Demographic cohorts create delayed consequences that drive historical dynamics.

## Demographic Representation

```text
DemographicState:
    total_population: int
    age_structure: AgeDistribution
    sex_structure: SexDistribution
    cohorts: [Cohort]
    fertility_rate: float
    mortality_rates: AgeSpecificRates
    migration_balance: float
    household_structure: HouseholdDistribution
```

### Cohort

```text
Cohort:
    birth_year: Time
    initial_size: int
    current_size: int
    sex_ratio: float
    trait_profile: TraitDistribution
    historical_events: [CohortEvent]
```

## Demographic Processes

### Fertility

Fertility depends on:
- biological fecundity
- mate availability
- social norms
- economic conditions
- health status
- age structure

### Pregnancy

Pregnancy is a biological and social process:
- gestation duration
- maternal health impact
- birth outcome risk
- social recognition

### Birth

Birth adds individuals to the population:
- infant health determined by genetics and gestation
- birth events are socially significant
- infant mortality is a major demographic factor

### Infant Mortality

Infant mortality has long-term consequences:
- high mortality reduces cohort size
- parental grief affects behavior
- population growth slows
- future labor supply decreases

### Household Formation

Households are social units:
- marriage, cohabitation, or kinship
- economic cooperation
- child rearing
- inheritance

### Migration

Migration changes geographic distribution:
- push factors (famine, conflict, disease)
- pull factors (opportunity, safety, resources)
- chain migration (following prior migrants)
- return migration

### Lifespan

Lifespan is the maximum age reached:
- determined by genetics, environment, and luck
- affects social structure (elderly presence)
- affects economic structure (experience retention)

## Cohort Effects

Cohort structure creates delayed consequences:

```text
child mortality crisis
    ↓
small cohort
    ↓
future labour shortage
    ↓
higher wages
    ↓
military recruitment problems
    ↓
inheritance concentration
```

Do not use global population modifiers when cohort structure matters.

## Demography and Other Domains

Demography interacts with:

- **Economy**: labor supply, consumer demand, inheritance
- **Society**: family structure, social organization, political power
- **Military**: recruitment pool, veteran population
- **Health**: disease burden, healthcare demand
- **Housing**: household formation drives construction
- **Education**: cohort size determines school demand

## Determinism

Demographic processes must be deterministic given:

- initial population structure
- fertility parameters
- mortality parameters
- migration parameters
- random stream (for stochastic aspects)

## Performance

Demographic data may be large. Strategies:

- Cohort aggregation for distant populations
- Event-driven updates for significant changes
- Statistical representation for common cases

## Related Documents

- `architecture.md` — biological system overview
- `reproduction.md` — fertility and birth
- `death.md` — mortality
- `aging.md` — age structure
- `populations.md` — population dynamics
- `docs/world/settlements.md` — settlement population

## TODO Categories

- `DEMO` — demography
- `BIO` — biology
- `WORLD` — general world systems
