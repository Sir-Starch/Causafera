# Attention

Attention is the mechanism that selects which information an agent processes. Cognition is bounded. Agents do not continuously recompute an optimal world model.

## Implemented Phase 7 foundation

`causafera-cognition::attention` implements the minimum bounded attention state accepted by RFC-PERCEPT-001:

- at most eight active foci and 64 candidates per update;
- fixed-point salience threshold and continuity bonus;
- deterministic ranking with subjective-ID tie-breaking;
- fixed-size active-state arrays;
- supporting subjective `PerceptId` references;
- agent-local `AttentionTargetId`, structurally distinct from `EntityId`, `BodySegmentId`, `PlaceId`, and `FeatureId`.

This primitive does not consume Ground Truth features or causal trace identities directly. Phase 9 scene mapping creates grounded subjective targets and keeps percept-to-trace correspondence in inaccessible external bookkeeping before attention participates in broader cognition. Goal relevance, learned salience, threat/opportunity interpretation, and habituation remain future processes rather than semantic target enums; Phase 10 prediction error is now available as a numeric downstream salience input.

## Attention Representation

```text
AttentionState:
    config: (capacity, salience_threshold, continuity_bonus)
    current_focus: bounded [AttentionTargetId]
    active_since: bounded [SimulationTime]
    supporting_percepts: bounded [PerceptId]
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
- `../rfc/RFC-COG-001.md` — accepted scene model and attention integration
- `../rfc/RFC-SCENE-001.md` — concrete bounded scene/continuity layout

## TODO Categories

- `COG` — cognition
- `MEM` — memory

## Phase 9 Integration Status

Attention foci now gate `SubjectiveScene` contents. Foci and cues meet only through agent-local `AttentionTargetId` and `PerceptId`; attention does not acquire authoritative identity or causal trace access. Numeric prediction error can later contribute salience through ordinary candidates without creating a semantic surprise kind.
