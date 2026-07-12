# Attention

Attention is the mechanism that selects which information an agent processes. Cognition is bounded. Agents do not continuously recompute an optimal world model.

## Attention Representation

```text
AttentionState:
    current_focus: [AttentionTarget]
    focus_capacity: float
    focus_duration: Time
    interruptibility: float
    attention_history: [AttentionEvent]
    salience_threshold: float
```

## Attention Targets

Agents may attend to:

- **Perceptual features**: detected patterns in sensation
- **Goals**: current objectives and subgoals
- **Threats**: potential dangers
- **Opportunities**: potential benefits
- **Social cues**: other agents' behavior
- **Internal state**: hunger, fatigue, pain
- **Memories**: recalled information
- **Predictions**: expected future events

## Attention Capacity

Attention is limited:

- **Single focus**: most agents can deeply attend to only one thing at a time
- **Divided attention**: limited capacity for parallel processing
- **Switching cost**: changing attention takes time and effort
- **Capacity variation**: fatigue, stress, and training affect capacity

## Attention Allocation

Attention allocation depends on:

- **Salience**: how noticeable a target is
- **Relevance**: how important to current goals
- **Novelty**: how unexpected
- **Threat level**: potential danger
- **Emotional state**: current arousal and valence
- **Habit**: learned attention patterns

## Attention and Perception

Attention determines what is perceived:

- **Selective perception**: attended features are processed more deeply
- **Inattentional blindness**: unattended features may be missed
- **Change blindness**: changes to unattended features may go unnoticed

## Attention and Action

Attention determines what actions are considered:

- **Goal-directed attention**: focus on goal-relevant information
- **Stimulus-driven attention**: reflexive response to salient stimuli
- **Habitual attention**: automatic focus on familiar patterns

## Attention and Social Interaction

Social attention includes:

- **Joint attention**: attending to what another attends to
- **Social monitoring**: tracking others' attention
- **Deception detection**: attending to inconsistency cues

## Determinism

Attention processes must be deterministic given:

- current attention state
- perceptual input
- goal state
- biological state
- memory state

## Performance

Attention simulation may be detailed for focus agents. Strategies:

- Simplified attention for distant or inactive agents
- Event-driven updates for attention shifts
- Aggregate representation for crowd attention

## Related Documents

- `salience.md` — what captures attention
- `memory.md` — attention determines what enters memory
- `prediction.md` — attention determines what is predicted
- `goals.md` — goals guide attention
- `habits.md` — habits automate attention
- `../architecture/cognition-rebaseline.md` — attention-driven subjective scene construction
- `../rfc/RFC-COG-001.md` — proposed scene model and attention integration

## TODO Categories

- `COG` — cognition
- `MEM` — memory
