# Development

Development is the process of biological and psychological change over the lifespan. Agents do not appear as fully formed adults with finished concepts and skills.

## Developmental Stages

Development proceeds through stages:

```text
DevelopmentState:
    current_stage: DevelopmentStage
    stage_progress: float
    physical_maturity: float
    cognitive_maturity: float
    social_maturity: float
    developmental_history: [DevelopmentalEvent]
```

### Candidate Stages

- **Prenatal**: gestation
- **Infancy**: 0-2 years
- **Childhood**: 2-12 years
- **Adolescence**: 12-18 years
- **Young adulthood**: 18-30 years
- **Maturity**: 30-50 years
- **Late adulthood**: 50+ years

Stage boundaries vary by lineage and individual.

## Physical Development

Physical development includes:

- **Growth**: increase in size and mass
- **Maturation**: development of reproductive capacity
- **Skill acquisition**: motor skills, coordination
- **Sensory development**: refinement of perceptual abilities

## Cognitive Development

Cognitive development includes:

- **Perceptual learning**: learning to extract features from sensation
- **Concept formation**: developing categories from experience
- **Language acquisition**: learning phonology, lexicon, grammar
- **Causal reasoning**: understanding cause and effect
- **Social cognition**: understanding others' minds

## Social Development

Social development includes:

- **Attachment**: bonding with caregivers
- **Imitation**: copying observed behavior
- **Social learning**: learning from others' experience
- **Apprenticeship**: structured skill transmission
- **Formal education**: institutionalized learning

## Development and Environment

Development is shaped by environment:

- **Nutrition**: affects growth and cognitive development
- **Stimulation**: affects neural development
- **Social interaction**: affects language and social cognition
- **Physical activity**: affects motor development
- **Stress**: affects emotional and cognitive development

Different developmental environments create different conceptual priors.

## Isekai Development

An isekai-born child retaining foreign memory may interact with local concept acquisition differently from an adult physical transfer:

- **Foreign concepts**: may have difficulty mapping to local categories
- **Foreign language**: may retain phonological patterns from Earth language
- **Foreign knowledge**: may apply Earth concepts to local phenomena
- **Social integration**: may develop hybrid identity

## Development and Other Domains

Development interacts with:

- **Physiology**: biological maturation enables cognitive development
- **Cognition**: developmental stage determines cognitive capacity
- **Language**: critical periods for language acquisition
- **Society**: social structures shape developmental opportunities
- **Economy**: resource availability determines nutrition and stimulation

## Determinism

Developmental processes must be deterministic given:

- genetic parameters
- environmental conditions
- developmental stage
- historical experiences

## Performance

Developmental simulation may be detailed for focus agents. Strategies:

- Simplified development for distant or inactive agents
- Aggregate representation for population-level processes
- Event-driven updates for developmental milestones

## Related Documents

- `architecture.md` — biological system overview
- `morphology.md` — physical growth
- `physiology.md` — physiological maturation
- `heredity.md` — genetic influences on development
- `docs/cognition/` — cognitive development
- `docs/language/` — language acquisition

## TODO Categories

- `BIO` — biology
- `COG` — cognition
- `LANG` — language
