# Biological Architecture

The biological subsystem provides compact multiscale biological representations for perception, health, development, reproduction, demographic history, biological variation, and local adaptation.

## Design Principles

### No Cellular Simulation

Do not simulate individual cells. The biological model operates at the tissue, organ, and organism level.

### Multiscale Representation

Biological entities are represented at multiple scales:

- **Organism**: whole individual with aggregate properties
- **Body system**: circulatory, nervous, digestive, etc.
- **Organ**: heart, liver, brain, etc.
- **Tissue**: muscle, bone, nerve, epithelium, etc.
- **Morphological segment**: limb, torso, head, etc.

### Causal State

Biological state is causal state. It must be capable of influencing and being influenced by other domains.

### Phase 5 Boundary

Phase 5 implements the label-free structural substrate in `causafera-biology`: typed body-segment identity, rooted parent topology, physical joint angle limits, fixed-point length, and relative orientation. Its completed pathogen-contract extension also defines property-based pathogen lineage ancestry, fixed-point transmission properties, canonically ordered objective host interactions, and causally referenced exposure records. Phase 6 now supplies generic provenance-aware proposal and commit contracts, but live infection, evolution, and host mutation remain unimplemented domain systems. The broader organism, body-system, development, physiology, health, and reproduction sketches below remain future architecture, not implemented authoritative types.

## Biological Representation

```text
BiologicalEntity:
    organism: OrganismState
    body_systems: [BodySystemState]
    morphology: MorphologyState
    physiology: PhysiologyState
    development: DevelopmentState
    heredity: HeredityState
    reproductive_state: ReproductiveState
    aging_state: AgingState
    health_state: HealthState
    immune_state: ImmuneState
```

### Organism State

```text
OrganismState:
    biological_lineage: PopulationLineageId
    sex: SexState
    age: Time
    vital_status: VitalStatus
    mass: float
    stature: float
    metabolic_rate: float
    temperature: float
```

### Body System State

```text
BodySystemState:
    system_type: SystemType
    functional_capacity: float
    damage_accumulation: float
    current_load: float
```

System types:
- Circulatory
- Respiratory
- Nervous
- Digestive
- Muscular
- Skeletal
- Immune
- Endocrine
- Reproductive

## Biological Lineages

Biological populations are represented as lineages with distributions of traits:

```text
PopulationLineage:
    lineage_id: PopulationLineageId
    trait_distributions: {TraitId → Distribution}
    geographic_range: Polygon
    historical_range: [RangeChange]
    genetic_diversity: float
```

Traits include:
- lifespan tendencies
- fertility
- development timing
- sensory ranges
- morphology
- metabolism
- mana coupling

Mana coupling is not a scalar magical aptitude and organisms contain no unexplained semantic mana resource. It is a contextual result of morphology, tissue properties, physiology, development, action, external field state, tools, geometry, and history. Mana may become physically retained in or around an organism as a carrier state, including during prenatal or other pre-birth development, but must retain transfer or conversion provenance and remain distinct from subjective “personal mana.” See `RFC-BIO-003`.

## Biological and Other Domains

Biology interacts with:

- **World**: geography determines local adaptation; climate determines health
- **Cognition**: biological state determines perceptual capacity; fatigue affects attention
- **Language**: biological development determines language acquisition capacity
- **Society**: biological variation may become socially categorized
- **Economy**: health affects labor capacity; reproduction affects future labor supply
- **Mana**: biological processes may interact with mana fields

## Determinism

Biological processes must be deterministic given:

- initial biological state
- environmental conditions
- genetic parameters
- random stream (for stochastic biological processes)

## Performance

Biological data may be large. Strategies:

- Compact representation for common cases
- Sparse representation for rare conditions
- Aggregate representation for distant or inactive organisms
- GPU acceleration for population-level processes

## Related Documents

- `morphology.md` — body structure
- `physiology.md` — biological processes
- `development.md` — growth and learning
- `heredity.md` — inheritance
- `reproduction.md` — reproduction
- `aging.md` — aging processes
- `death.md` — death and termination
- `pathogens.md` — disease and infection
- `populations.md` — biological populations
- `demography.md` — population dynamics

## RFCs

- `RFC-BIO-001: Minimal Biological Structural Model` — Accepted and implemented for Phase 5 structure
- `RFC-BIO-002: Minimal Pathogen Contracts` — Accepted and implemented for the Phase 5 pathogen extension

## TODO Categories

- `BIO` — biology
- `DEMO` — demography
- `PATH` — pathogens
