# Settlements

Settlements are concentrations of human habitation. They emerge from geographic, economic, and social factors rather than being placed by generation algorithms.

## Settlement Formation

Settlements form where:

- water is available
- terrain is buildable
- agriculture or trade is viable
- defense is possible
- resources are accessible

The simulation must not place settlements arbitrarily. They must be traceable to geographic and economic causality.

## Settlement Representation

```text
Settlement:
    location: WorldCoord
    founding_date: Time
    founding_reason: FoundingCause
    population: PopulationState
    parcels: [ParcelId]
    structures: [StructureId]
    infrastructure: InfrastructureState
    economy: EconomicState
    social_organization: SocialState
    historical_events: [HistoricalEvent]
```

## Settlement Types

Settlement types are observer classifications. Ground Truth stores:

- size
- density
- economic activities
- building types
- infrastructure
- social organization

Common observer types:
- Hamlet
- Village
- Town
- City
- Metropolis

## Settlement and Geography

Settlements interact with geography:

- **Terrain**: flat ground for construction; elevated ground for defense
- **Hydrology**: water for drinking, agriculture, transport
- **Climate**: growing season length; extreme weather risk
- **Geology**: building materials; foundation stability
- **Ecology**: food sources; building materials; disease risk

## Settlement Growth

Settlements grow through:

- **Natural increase**: birth exceeding death
- **Migration**: in-migration exceeding out-migration
- **Economic expansion**: new activities attracting population
- **Infrastructure development**: improved conditions supporting density

Growth is constrained by:
- geographic limits (water, buildable land)
- economic limits (employment, food supply)
- social limits (organization capacity, conflict)
- health limits (disease, sanitation)

## Settlement Decline

Settlements may decline due to:

- **Resource depletion**: water, soil, minerals exhausted
- **Economic shift**: trade routes change; activities become obsolete
- **Environmental change**: climate shift; natural disaster
- **Conflict**: war; social breakdown
- **Disease**: epidemic reducing population
- **Political change**: administrative reorganization

## Urban Infrastructure

See `docs/city/` for detailed infrastructure documentation:

- `parcels.md` — land division
- `buildings.md` — construction
- `streets.md` — transport networks
- `infrastructure-networks.md` — water, sewage, waste
- `maintenance.md` — upkeep and decay
- `urban-growth.md` — expansion patterns
- `fire.md` — fire risk and response

## Determinism

Settlement evolution must be deterministic given:

- initial conditions
- geographic constraints
- economic parameters
- social parameters

## Performance

Settlement data may be large. Strategies:

- Spatial chunking
- Level of detail representation
- Aggregate distant settlements
- Cache stable settlement state

## Related Documents

- `geography-philosophy.md` — geographic causality
- `spatial-hierarchy.md` — spatial organization
- `docs/city/` — urban infrastructure and growth
- `docs/biology/demography.md` — population dynamics
- `docs/society/` — social organization

## TODO Categories

- `CITY` — city and settlement systems
- `WORLD` — general world systems
- `INFRA` — infrastructure
