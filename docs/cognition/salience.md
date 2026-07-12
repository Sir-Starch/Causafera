# Salience

Salience is the property of being noticeable or attention-grabbing. It determines what captures agent attention in a complex environment.

## Salience Representation

```text
SalienceState:
    current_salience_map: SalienceMap
    salience_threshold: float
    habituation_state: HabituationState
    surprise_history: [SurpriseEvent]
```

## Salience Factors

Something is salient if it is:

- **Novel**: unexpected, unfamiliar
- **Intense**: strong sensory signal
- **Contrastive**: different from surroundings
- **Moving**: changing, dynamic
- **Threatening**: potentially dangerous
- **Rewarding**: potentially beneficial
- **Socially relevant**: involving other agents
- **Goal-relevant**: related to current objectives
- **Emotionally charged**: arousing strong feeling

## Salience and Attention

Salience guides attention allocation:

- High salience items capture attention automatically
- Low salience items may be missed even if important
- Salience competition determines what is attended
- Habituation reduces salience of repeated stimuli

## Habituation

Habituation is the reduction in response to repeated stimuli:

- **Stimulus-specific**: habituation to one stimulus does not generalize
- **Dishabituation**: novel stimulus restores response
- **Spontaneous recovery**: response returns after delay

Habituation allows agents to ignore constant background and focus on change.

## Surprise

Surprise occurs when expectation is violated:

- **Prediction error**: outcome differs from expectation
- **Novelty**: no expectation exists
- **Incongruity**: stimulus does not fit current context

Surprise increases salience and drives learning.

## Salience and Learning

Salient events are better learned:

- **Emotional salience**: emotional events are better remembered
- **Novelty salience**: novel events are better remembered
- **Social salience**: social events are better remembered

## Salience and Culture

Salience is partly culturally determined:

- **Learned salience**: what a culture emphasizes becomes salient
- **Social attention**: what others attend to becomes salient
- **Language**: vocabulary highlights certain distinctions

## Determinism

Salience processes must be deterministic given:

- current perceptual input
- expectation state
- habituation state
- goal state
- emotional state

## Performance

Salience computation may be frequent. Strategies:

- Efficient salience computation for common cases
- Batch processing for multiple stimuli
- Sparse updates for stable environments

## Related Documents

- `attention.md` — attention allocation
- `memory.md` — salient events are better remembered
- `prediction.md` — surprise is prediction error
- `belief-inertia.md` — salient evidence may overcome inertia

## TODO Categories

- `COG` — cognition
- `MEM` — memory
