# Prediction

Prediction is the generation of expectations about future events. It guides action, drives learning, and shapes perception.

## Prediction Representation

```text
PredictionState:
    current_predictions: [Prediction]
    prediction_confidence: float
    prediction_horizon: Time
    error_history: [PredictionError]
    model_quality: float
```

### Prediction

```text
Prediction:
    predicted_event: EventDescription
    predicted_time: Time
    confidence: float
    basis: PredictionBasis
    alternatives: [AlternativePrediction]
```

## Prediction Types

- **Sensory prediction**: what will be perceived next
- **Action outcome**: what will result from an action
- **Social prediction**: what another agent will do
- **Environmental prediction**: how the world will change
- **Temporal prediction**: when events will occur

## Prediction Mechanisms

### Pattern Completion

Agents predict by completing familiar patterns:

- **Sequence prediction**: what comes next in a sequence
- **Spatial prediction**: what lies beyond the visible
- **Causal prediction**: what follows from a cause

### Model-Based Prediction

Agents predict using causal models:

- **Physical model**: objects move according to physics
- **Social model**: agents act according to goals
- **Biological model**: organisms behave according to needs

### Statistical Prediction

Agents predict from frequency:

- **Base rate**: how common is this outcome
- **Conditional probability**: how likely given this context
- **Regression to mean**: extreme values tend toward average

## Prediction Error

Prediction error is the difference between prediction and outcome:

- **Positive error**: outcome better than predicted
- **Negative error**: outcome worse than predicted
- **Surprise**: no prediction existed

Prediction error drives learning:
- large errors update models more
- consistent errors indicate model failure
- random errors indicate unpredictability

## Prediction and Action

Predictions guide action selection:

- **Expected utility**: choose action with best predicted outcome
- **Risk assessment**: avoid actions with bad predicted outcomes
- **Planning**: sequence actions to achieve predicted goal state

## Prediction and Perception

Predictions shape perception:

- **Top-down processing**: expectations influence what is perceived
- **Predictive coding**: perception is prediction error minimization
- **Confirmation bias**: predictions bias interpretation

## Prediction and Belief

Predictions are related to beliefs:

- **Belief-based prediction**: predictions derive from beliefs
- **Prediction-based belief**: beliefs are confirmed predictions
- **Belief inertia**: existing beliefs resist prediction error

## Determinism

Prediction processes must be deterministic given:

- current prediction state
- current beliefs
- current observations
- current goals
- learning history

## Performance

Prediction computation may be frequent. Strategies:

- Cached predictions for stable situations
- Simplified prediction for routine cases
- Detailed prediction for significant decisions

## Related Documents

- `attention.md` — attention to prediction-relevant information
- `memory.md` — memory provides prediction basis
- `salience.md` — prediction error creates surprise
- `belief-inertia.md` — beliefs resist prediction error
- `goals.md` — goals guide prediction relevance

## TODO Categories

- `COG` — cognition
- `BELIEF` — belief systems

## Phase 10 Implementation Status

`PredictiveState` stores at most eight generic signature expectations. Due predictions compare against accessible identity-free cues and emit explicit fixed-point `PredictionError` records with subjective percept support. It does not run physics, classify the event, or access Ground Truth.

`AgencyModel` learns bounded associations between opaque action and outcome pattern IDs through deterministic proximity-weighted updates. These associations may be wrong. `TemporalEnvelope` retains at most eight recent subjective frames and aggregate prediction error, keeping immediate continuity separate from autobiographical memory.

Phase 12 uses prediction-error magnitude only as a numeric learning signal for directed subjective causal hypotheses. It does not convert an error into an objective cause or semantic event classification.
