# Aging

Aging is the progressive decline in biological function over time. It affects all organisms and creates the temporal structure of populations.

## Aging Representation

```text
AgingState:
    chronological_age: Time
    biological_age: float
    functional_reserve: float
    damage_accumulation: float
    repair_capacity: float
    senescence_markers: [SenescenceMarker]
    mortality_risk: float
```

## Aging Processes

### Damage Accumulation

Over time, organisms accumulate damage:

- **Cellular**: mutation, protein damage, telomere shortening
- **Tissue**: fibrosis, atrophy, calcification
- **Organ**: reduced function, structural change
- **System**: decreased capacity, reduced reserve

### Repair Decline

Repair capacity decreases with age:

- **Regeneration**: slower wound healing
- **Immune response**: reduced pathogen clearance
- **DNA repair**: increased mutation accumulation
- **Protein maintenance**: increased damage accumulation

### Functional Reserve

Functional reserve is the capacity to respond to stress:

- **Young**: high reserve, rapid recovery
- **Mature**: moderate reserve, adequate recovery
- **Old**: low reserve, slow recovery
- **Very old**: minimal reserve, high vulnerability

## Aging and Mortality

Aging increases mortality risk:

- **Intrinsic**: biological failure (heart, brain, immune system)
- **Extrinsic**: increased vulnerability to disease, injury, stress
- **Catastrophic**: sudden failure of critical system

Mortality risk may be modified by:
- nutrition
- activity level
- social support
- medical care
- environmental conditions

## Aging and Society

Societies respond to aging through:

- **Care**: support for elderly individuals
- **Inheritance**: transfer of resources to younger generation
- **Wisdom**: valuation of elderly knowledge and experience
- **Ritual**: ceremonies marking life stages
- **Exclusion**: marginalization of elderly individuals

These responses are emergent, not primitive.

## Aging and Other Domains

Aging interacts with:

- **Physiology**: declining function affects all systems
- **Cognition**: cognitive decline affects decision-making
- **Society**: age structure affects social organization
- **Economy**: elderly individuals may be producers or dependents
- **Demography**: aging affects population structure
- **Mana**: long-lived individuals may accumulate mana-related changes

## Determinism

Aging processes must be deterministic given:

- initial biological state
- chronological age
- environmental conditions
- damage history

## Performance

Aging is a slow process. Strategies:

- Infrequent updates for stable periods
- Event-driven updates for significant changes
- Aggregate representation for distant populations

## Related Documents

- `architecture.md` — biological system overview
- `physiology.md` — functional decline
- `morphology.md` — physical degeneration
- `death.md` — mortality and termination
- `demography.md` — population aging

## TODO Categories

- `BIO` — biology
- `DEMO` — demography
