# Reproduction

Reproduction is the creation of new biological entities. It maintains populations, transmits traits, and creates the generational structure that underlies demographic history.

## Reproductive Representation

```text
ReproductiveState:
    sex: SexState
    fertility: FertilityState
    reproductive_history: [ReproductiveEvent]
    current_pregnancy: Option<PregnancyState>
    mate_preferences: PreferenceSet
    social_constraints: SocialConstraintSet
```

### Fertility State

```text
FertilityState:
    fecundity: float
    fertility_window: (Time, Time)
    current_fertility: float
    fertility_modifiers: [FertilityModifier]
```

### Pregnancy State

```text
PregnancyState:
    gestation_start: Time
    expected_birth: Time
    fetus_state: FetusState
    maternal_health_impact: float
    complications: [Complication]
```

## Reproductive Processes

### Mating

Mating requires:
- compatible fertility windows
- social opportunity
- mate selection
- successful conception

Mate selection may involve:
- physical assessment
- social status
- resource availability
- familial arrangement
- personal preference

### Gestation

Gestation is the period between conception and birth:

- duration varies by lineage
- maternal health affects outcome
- environmental conditions affect development
- complications may occur

### Birth

Birth produces a new biological entity:

- initial state determined by genetics and gestation conditions
- maternal health affects birth outcome
- complications may affect infant survival
- birth events are significant social occasions

## Reproduction and Demography

Reproduction drives demography:

- **Fertility rate**: births per reproductive individual
- **Generation time**: average age at reproduction
- **Population growth**: births minus deaths
- **Age structure**: distribution of ages in population

## Reproduction and Society

Societies regulate reproduction through:

- **Marriage**: social recognition of reproductive partnership
- **Inheritance**: transmission of property to offspring
- **Lineage**: tracking of descent
- **Taboo**: restrictions on mating
- **Fertility control**: practices to limit or encourage reproduction

These social structures are emergent, not primitive.

## Reproduction and Other Domains

Reproduction interacts with:

- **Heredity**: transmits traits to offspring
- **Development**: gestation is early development
- **Demography**: births determine population change
- **Economy**: children require resources; adults produce them
- **Society**: reproduction creates kinship structure
- **Health**: pregnancy affects maternal health; birth affects infant health

## Determinism

Reproductive outcomes must be deterministic given:

- parent biological states
- environmental conditions
- social conditions
- random stream (for stochastic aspects)

## Performance

Reproduction events are relatively rare. Strategies:

- Event-driven processing
- Batch fertility updates
- Aggregate representation for distant populations

## Related Documents

- `architecture.md` — biological system overview
- `heredity.md` — trait transmission
- `development.md` — gestation and early development
- `demography.md` — population-level reproduction
- `docs/biology/populations.md` — population dynamics

## TODO Categories

- `BIO` — biology
- `DEMO` — demography
