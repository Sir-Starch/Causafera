# Habits

Habits are automatic behavior patterns triggered by context. They reduce cognitive load and create stable behavioral structures.

## Habit Representation

```text
HabitState:
    habits: [Habit]
    habit_strength: float
    context_sensitivity: float
    automaticity: float
    habit_history: [HabitEvent]
```

### Habit

```text
Habit:
    habit_id: HabitId
    trigger_context: ContextDescription
    behavior_sequence: [Action]
    reinforcement_history: [ReinforcementEvent]
    strength: float
    automaticity: float
    resistance_to_change: float
```

## Habit Formation

Habits form through repetition:

- **Initial behavior**: deliberate action for goal
- **Repetition**: repeated in same context
- **Reinforcement**: positive outcome strengthens association
- **Automaticity**: behavior becomes automatic
- **Context dependence**: behavior triggered by context alone

## Habit and Goals

Habits relate to goals:

- **Goal-derived**: habit originally served a goal
- **Goal-independent**: habit persists without current goal
- **Goal-conflicting**: habit contradicts current goal
- **Goal-supporting**: habit automatically advances goal

## Habit and Attention

Habits reduce attention requirements:

- **Automatic execution**: minimal attention required
- **Parallel processing**: habit executes while attention elsewhere
- **Interruption**: unexpected events may interrupt habit
- **Error**: inattention may lead to habit errors

## Habit and Social Structure

Shared habits create social structure:

- **Synchronization**: coordinated habits create collective patterns
- **Tradition**: transmitted habits create cultural continuity
- **Identity**: distinctive habits mark group membership
- **Institution**: organized habits create organizations

## Habit and Magic

In an information-sensitive magical world, habits have physical consequences:

- **Repetition**: habitual behavior creates repeated patterns
- **Synchronization**: shared habits create coordinated patterns
- **Persistence**: long habits create stable patterns
- **Mana response**: these patterns may affect mana fields

A habitual practice may become magically significant through repetition alone.

## Habit Change

Habits change through:

- **Extinction**: behavior no longer reinforced
- **Replacement**: new habit replaces old
- **Context change**: trigger context no longer occurs
- **Conscious override**: deliberate inhibition
- **Social pressure**: others discourage habit

## Determinism

Habit processes must be deterministic given:

- current habit state
- context
- reinforcement history
- goal state
- biological state

## Performance

Habit execution is efficient. Strategies:

- Fast habit lookup by context
- Automatic execution without deliberation
- Minimal memory access

## Related Documents

- `attention.md` — habits reduce attention load
- `memory.md` — habits are procedural memories
- `goals.md` — habits originally serve goals
- `belief-inertia.md` — habits resist change
- `trust.md` — trusted models shape habit formation

## TODO Categories

- `COG` — cognition
- `BELIEF` — belief systems
