# Goals

Goals are desired future states that guide agent behavior. They create the directionality that makes behavior meaningful.

## Goal Representation

```text
GoalState:
    active_goals: [Goal]
    goal_hierarchy: GoalHierarchy
    goal_conflicts: [GoalConflict]
    goal_history: [GoalEvent]
    motivation_state: MotivationState
```

### Goal

```text
Goal:
    goal_id: GoalId
    desired_state: StateDescription
    priority: float
    deadline: Option<Time>
    subgoals: [GoalId]
    progress: float
    activation_conditions: [Condition]
    termination_conditions: [Condition]
```

## Goal Types

- **Survival goals**: food, water, shelter, safety
- **Social goals**: affiliation, status, mating
- **Material goals**: wealth, possessions, tools
- **Cognitive goals**: understanding, prediction, mastery
- **Aesthetic goals**: beauty, harmony, order
- **Moral goals**: justice, fairness, duty
- **Identity goals**: self-expression, consistency, growth

## Goal Formation

Goals form through:

- **Biological need**: hunger creates food goal
- **Social pressure**: status competition creates status goal
- **Causal inference**: understanding creates knowledge goal
- **Imitation**: copying others' goals
- **Instruction**: adopting taught goals
- **Reinterpretation**: giving new meaning to existing goals

## Goal Hierarchy

Goals are organized hierarchically:

- **Superordinate**: abstract, long-term ("become a master baker")
- **Intermediate**: medium-term ("learn sourdough technique")
- **Subordinate**: concrete, immediate ("mix flour and water")

## Goal Conflict

Goals may conflict:

- **Resource conflict**: two goals require same resource
- **Temporal conflict**: goals require same time
- **Value conflict**: goals imply incompatible values
- **Social conflict**: goals conflict with others' goals

Conflict resolution depends on:
- priority
- urgency
- expected success
- social context
- emotional state

## Goal and Action

Goals drive action selection:

- **Means-end analysis**: what actions achieve the goal
- **Planning**: sequencing actions
- **Opportunism**: taking unexpected opportunities
- **Persistence**: continuing despite obstacles
- **Abandonment**: giving up when success seems impossible

## Goal and Prediction

Goals guide prediction:

- **Relevance filtering**: predict goal-relevant events
- **Outcome evaluation**: predict whether actions achieve goals
- **Risk assessment**: predict threats to goals

## Determinism

Goal processes must be deterministic given:

- current goal state
- biological needs
- social context
- belief state
- memory state
- environmental opportunities

## Performance

Goal computation may be frequent. Strategies:

- Cached goal hierarchies for stable situations
- Event-driven updates for significant changes
- Simplified goals for routine behavior

## Related Documents

- `attention.md` — attention to goal-relevant information
- `memory.md` — memory stores goal history
- `prediction.md` — prediction of goal outcomes
- `habits.md` — habits automate goal pursuit
- `trust.md` — trust affects goal adoption

## TODO Categories

- `COG` — cognition
- `BELIEF` — belief systems
