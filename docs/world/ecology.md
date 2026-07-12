# Ecology

Ecology is the distribution and interaction of living organisms in the world. It determines food availability, disease reservoirs, materials, and many other simulation outcomes.

## Ecological Representation

Ecology is represented at multiple scales:

```text
EcologicalState:
    biome: BiomeClassification
    vegetation: VegetationLayer
    animal_populations: [AnimalPopulation]
    soil_ecology: SoilCommunity
    disease_reservoirs: [DiseaseReservoir]
    ecological_processes: [EcologicalProcess]
```

### Biome

A biome is a large ecological unit characterized by:

- dominant vegetation type
- climate association
- typical species assemblage
- productivity

Biomes are observer classifications, not authoritative simulation categories. Ground Truth stores:

- vegetation cover
- species presence
- productivity
- soil properties

### Vegetation Layer

```text
VegetationLayer:
    canopy: [PlantPopulation]
    understory: [PlantPopulation]
    ground_cover: [PlantPopulation]
    root_community: RootCommunity
```

### Animal Population

```text
AnimalPopulation:
    species_lineage: SpeciesLineageId
    population_size: int
    range: Polygon
    density: ScalarField
    migration_pattern: MigrationPattern
    predator_prey_relationships: [EcologicalInteraction]
```

## Ecological Processes

The simulation models:

- **Primary production**: plant growth
- **Consumption**: herbivory, predation
- **Decomposition**: nutrient cycling
- **Competition**: resource limitation
- **Succession**: community change over time
- **Migration**: range shifts
- **Extinction**: local or global population loss

## Ecology and Other Domains

Ecology interacts with:

- **Climate**: temperature and precipitation determine biome
- **Terrain**: elevation and slope determine vegetation
- **Hydrology**: water availability determines productivity
- **Geology**: soil fertility determines plant communities
- **Biology**: species are biological populations
- **Disease**: animals and plants may be disease reservoirs
- **Economy**: ecological resources support material extraction
- **Mana**: ecological patterns may interact with mana fields

## Species and Lineages

Species are observer classifications. Ground Truth stores:

- biological population lineages
- morphological traits
- physiological traits
- genetic similarity
- reproductive compatibility

Different societies may classify the same biological continuum into different species.

## Ecological Change

Ecology changes over time:

- **Succession**: gradual community change after disturbance
- **Invasion**: new species entering an ecosystem
- **Extinction**: species loss
- **Climate tracking**: range shifts with climate change
- **Human impact**: hunting, agriculture, habitat modification

## Determinism

Ecological generation must be deterministic given:

- world_seed
- climate state
- terrain state
- geological state
- hydrological state

## Performance

Ecological data may be large. Strategies:

- Represent populations rather than individuals where possible
- Use statistical distributions for common species
- Track rare or significant species individually
- GPU acceleration for spatial distribution operations

## Related Documents

- `geography-philosophy.md` — geographic causality
- `climate.md` — climate determination of biomes
- `terrain.md` — terrain effects on ecology
- `hydrology.md` — water availability
- `biology/populations.md` — biological population representation
- `world-generation-provenance.md` — provenance tracking

## TODO Categories

- `ECO` — ecology
- `WORLD` — general world systems
- `BIO` — biology
