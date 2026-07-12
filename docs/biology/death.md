# Death

Death is the termination of biological function. It is a permanent state transition that removes an organism from the active simulation.

## Death Representation

```text
DeathState:
    time_of_death: Time
    cause_of_death: DeathCause
    mechanism: DeathMechanism
    location: WorldCoord
    body_state: BodyState
    historical_significance: float
```

## Causes of Death

Death may result from:

- **Disease**: pathogen infection, organ failure
- **Injury**: trauma, accident, violence
- **Starvation**: inadequate nutrition
- **Exposure**: extreme temperature, environmental hazard
- **Predation**: attack by animal
- **Aging**: biological system failure
- **Suicide**: self-inflicted death
- **Infanticide**: death of infant by parent or other

## Death Process

### Terminal State

Before death, organisms enter a terminal state:

- **Critical injury**: severe trauma with low survival probability
- **Terminal illness**: progressive disease with fatal prognosis
- **Extreme age**: multiple system failure

### Death Event

Death is an event that:

- terminates biological processes
- removes agent from active decision-making
- creates a corpse
- triggers social responses

### Post-Death

After death:

- **Corpse**: physical remains with decomposition timeline
- **Estate**: property and obligations requiring transfer
- **Social memory**: remembrance by surviving individuals
- **Historical record**: documentation of life and death

## Death and Society

Societies respond to death through:

- **Mourning**: emotional and social response
- **Funeral**: disposal of corpse, ritual
- **Inheritance**: transfer of property
- **Memory**: preservation of identity in social memory
- **Ancestor veneration**: continued social relationship with dead

These practices are emergent, not primitive.

## Death and Demography

Death is a primary demographic process:

- **Mortality rate**: deaths per population per time
- **Life expectancy**: average age at death
- **Age-specific mortality**: death risk by age
- **Cause-specific mortality**: deaths by cause

## Death and Other Domains

Death interacts with:

- **Demography**: deaths determine population change
- **Economy**: death affects labor supply and inheritance
- **Society**: death creates kinship gaps and social reorganization
- **Ecology**: corpses become nutrient sources
- **Mana**: death may create or alter mana patterns
- **Metaphysics**: death raises questions about identity persistence

## Determinism

Death must be deterministic given:

- biological state
- environmental conditions
- injury and disease state
- random stream (for stochastic aspects)

## Performance

Death events are relatively rare. Strategies:

- Event-driven processing
- Batch mortality updates for populations
- Aggregate representation for distant deaths

## Related Documents

- `architecture.md` — biological system overview
- `physiology.md` — functional failure
- `aging.md` — age-related mortality
- `pathogens.md` — disease mortality
- `demography.md` — population mortality
- `docs/metaphysics/death-and-persistence.md` — identity after death

## TODO Categories

- `BIO` — biology
- `DEMO` — demography
- `META` — metaphysics
