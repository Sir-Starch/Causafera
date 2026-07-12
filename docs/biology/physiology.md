# Physiology

Physiology is the study of biological function. It describes how organisms maintain life, process energy, respond to stimuli, and perform activities.

## Physiological Representation

Physiology is represented as the functional state of body systems:

```text
PhysiologyState:
    metabolism: MetabolismState
    circulation: CirculationState
    respiration: RespirationState
    nervous_system: NervousSystemState
    digestion: DigestionState
    immune_function: ImmuneState
    endocrine_state: EndocrineState
    sensory_state: SensoryState
    motor_state: MotorState
    homeostasis: HomeostasisState
```

### Metabolism

```text
MetabolismState:
    basal_rate: float
    current_rate: float
    energy_reserves: float
    nutrient_status: {Nutrient → float}
    hydration: float
    temperature_regulation: float
```

### Circulation

```text
CirculationState:
    heart_rate: float
    blood_pressure: (float, float)
    blood_volume: float
    oxygen_delivery: float
    nutrient_delivery: float
    waste_removal: float
```

### Nervous System

```text
NervousSystemState:
    arousal_level: float
    fatigue_accumulation: float
    pain_signals: [PainSignal]
    reflex_state: ReflexState
    coordination: float
```

## Physiological Processes

### Energy Balance

```text
energy_intake - energy_expenditure = energy_reserve_change
```

Energy expenditure includes:
- basal metabolism
- physical activity
- thermoregulation
- growth and repair
- immune response

### Homeostasis

Organisms maintain internal stability:

- temperature
- pH
- fluid balance
- electrolyte balance
- blood pressure

Homeostatic mechanisms respond to:
- environmental change
- activity level
- injury
- disease
- psychological state

### Stress Response

Physiological stress response includes:

- arousal increase
- heart rate elevation
- energy mobilization
- immune modulation
- sensory sharpening

Chronic stress may cause:
- immune suppression
- metabolic dysregulation
- cardiovascular damage
- cognitive impairment

## Physiology and Other Domains

Physiology interacts with:

- **Cognition**: fatigue affects attention; arousal affects perception
- **Behavior**: energy availability determines activity capacity
- **Health**: physiological state determines disease susceptibility
- **Environment**: temperature, altitude, toxins affect physiology
- **Mana**: physiological processes may interact with mana fields

## Physiological Observation

Agents observe physiological state indirectly:

- **Self-observation**: fatigue, hunger, pain, temperature
- **Other-observation**: posture, movement, skin color, breathing rate
- **Instrumental**: measurement with tools (where available)

Agents construct concepts such as "fever" or "weakness" from observed physiological signs. These concepts are subjective, not authoritative.

## Determinism

Physiological processes must be deterministic given:

- current physiological state
- environmental conditions
- activity level
- injury and disease state

## Performance

Physiological simulation may be detailed. Strategies:

- Simplified physiology for distant or inactive organisms
- Aggregate representation for population-level processes
- Event-driven updates for stable states
- Batch processing for similar organisms

## Related Documents

- `architecture.md` — biological system overview
- `morphology.md` — physical structure
- `development.md` — physiological development
- `pathogens.md` — disease effects on physiology
- `aging.md` — physiological decline

## TODO Categories

- `BIO` — biology
