# Belief Inertia

Belief inertia is the resistance to changing beliefs in the face of new evidence. Stable mistakes are essential to Ontopolis.

## Belief Inertia Representation

```text
BeliefInertiaState:
    belief_stability: float
    evidence_threshold: float
    confirmation_bias: float
    disconfirmation_sensitivity: float
    belief_age: Time
    social_reinforcement: float
```

## Sources of Inertia

Beliefs resist change due to:

- **Confirmation bias**: seeking confirming evidence
- **Disconfirmation avoidance**: ignoring contradicting evidence
- **Sunk cost**: investment in belief creates commitment
- **Social reinforcement**: others share the belief
- **Identity**: belief is part of self-concept
- **Explanation coherence**: belief fits with other beliefs
- **Source trust**: belief came from trusted source
- **Repetition**: belief has been stated many times

## Inertia and Evidence

Evidence affects beliefs through:

- **Strength**: strong evidence overcomes more inertia
- **Salience**: salient evidence is more influential
- **Consistency**: consistent evidence is more influential
- **Source**: trusted sources overcome more inertia
- **Timing**: recent evidence is more influential

## Inertia and Social Transmission

Social processes reinforce inertia:

- **Echo chambers**: exposure to confirming beliefs
- **Authority**: respected figures maintain beliefs
- **Tradition**: historical continuity supports beliefs
- **Ritual**: repeated practice reinforces beliefs
- **Institutionalization**: organizations perpetuate beliefs

## Inertia and Change

Beliefs may change through:

- **Cumulative evidence**: gradual accumulation of contradicting evidence
- **Crisis**: dramatic event contradicting belief
- **Social shift**: community changes belief
- **Replacement**: new belief replaces old
- **Reinterpretation**: old belief given new meaning

## Inertia and Magic

In an information-sensitive magical world, belief inertia has physical consequences:

- **Repeated practice**: stable beliefs create repeated behavior
- **Synchronized behavior**: shared beliefs create coordinated action
- **Persistent patterns**: long-held beliefs create stable structures
- **Mana response**: these patterns may affect mana fields

A false belief may become physically reinforced through the behavior it creates.

## Determinism

Belief inertia processes must be deterministic given:

- current belief state
- evidence history
- social context
- personality parameters
- biological state

## Performance

Belief inertia computation may be frequent. Strategies:

- Cached inertia values for stable beliefs
- Event-driven updates for significant evidence
- Aggregate representation for widely shared beliefs

## Related Documents

- `attention.md` — attention to confirming evidence
- `memory.md` — memory stores belief-supporting evidence
- `salience.md` — salient evidence may overcome inertia
- `prediction.md` — predictions based on beliefs
- `trust.md` — trust in belief sources
- `goals.md` — goals may motivate belief maintenance

## TODO Categories

- `COG` — cognition
- `BELIEF` — belief systems

## Phase 12 Implementation Status

`BeliefStore` holds at most 32 beliefs over opaque subjective `ConceptId` subjects. Confidence and inertia use the shared fixed-point cognitive weight; updates accept canonical batches of at most 32 evidence records. Evidence carries only an opaque belief reference, signed support direction, numeric strength/salience, a subjective source hypothesis, and percept support.

Evidence is weighted by `TrustStore` before explicit inertia is applied, so weak contradiction can leave a high-inertia mistake stable. `CausalHypothesisStore` separately retains at most 32 directed associations between opaque subjective pattern IDs using proximity and prediction error. Neither store queries objective truth or authoritative entity identity.
