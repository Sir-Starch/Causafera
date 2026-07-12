# RFC-COG-001: Subjective Scene and Cognitive Continuity Model

**Status:** Accepted

## Summary

Define the minimum viable representation and processing model for the subjective scene construction layer that sits between generic perceptual feature extraction and subjective concept formation / belief. The model must support perceived object persistence, body schema, self-model, active working context, prediction, prediction error, episodic memory reactivation, agency attribution, and subjective temporal continuity without simulating a complete human brain.

## Motivation

The current architecture describes:

1. Ground Truth → physical access → sensory acquisition;
2. Generic perceptual feature extraction;
3. Subjective concepts and beliefs.

It lacks an explicit layer for how an agent constructs a coherent, transient, agent-specific model of the currently experienced situation from scattered perceptual input. Without this layer, future implementers will naturally pass `Feature` lists directly into concept formation, causing agents to implicitly know authoritative entity identity and to lack situated experience.

This RFC investigates the minimum viable representation for that missing layer. It does not design a complete cognitive architecture. It establishes the boundary, the required concepts, and the performance constraints within which a future implementation must operate.

## Design Principles

1. **Sparse.** Only a small subset of possible information is active at any time.
2. **Active-set driven.** What is active depends on current attention, salience, and prediction error.
3. **Multi-rate where useful.** Some elements update every tick; others update only when cued.
4. **Data-oriented.** Storage layout must support cache-friendly batch iteration over active agents.
5. **Bounded in active cognitive state.** The total size of the subjective scene for one agent must have a deterministic upper bound.
6. **Compatible with deterministic execution.** All activation, similarity matching, and prediction must be deterministic given the same inputs and history.
7. **Not a brain simulation.** We model the functional boundary, not neurobiology.

## Minimum Viable Representation

### Subjective Scene Model

The subjective scene is a transient, agent-specific structure representing the currently experienced situation. It is reconstructed each cognitive tick from:

- newly extracted generic features (via sensory acquisition);
- current body schema state;
- current self-model state;
- active working context;
- predictions and prediction errors;
- reactivated episodic memories;
- current goals and needs;
- attention focus.

**Candidate contents (not a final struct):**

```text
SubjectiveScene:
    perceived_self_state: PerceivedSelfState
    body_schema: BodySchemaState
    believed_location: SubjectiveLocation
    perceived_objects: [PerceivedObject]
    perceived_agents: [PerceivedAgent]
    subjective_relations: [SubjectiveSpatialRelation]
    active_concepts: [ActiveConcept]
    active_memories: [ActiveMemory]
    active_goals: [ActiveGoal]
    predictions: [NearFuturePrediction]
    attention_focus: AttentionFocus
    uncertainty_map: UncertaintyMap
```

**Constraints:**

- The scene must have a bounded size. A candidate bound: the scene may not contain more than N perceived objects, M active memories, and K active concepts per agent, where N, M, K are architecture constants.
- The scene is not persisted as authoritative state. It is recomputed each tick from the agent's persistent memory and current perceptual input. Loss of the scene between ticks is acceptable because the persistent store retains what matters.
- The scene may contain inferred objects (e.g., an occluded object believed to still exist behind a wall) only when inference from accessible information supports that belief.

### Perceived Object Persistence

Agents track objects subjectively. The authoritative engine knows `EntityId(81)`. The agent maintains `PerceivedObjectIdentity(19)`.

**Candidate representation:**

```text
PerceivedObject:
    perceived_identity: PerceivedObjectIdentity
    same_as_previous_confidence: float
    continuity_confidence: float
    expected_location: SubjectiveLocation
    appearance_signature: AppearanceSignature
    relationship_associations: [RelationshipAssociation]
    identity_confidence: float
    last_observed_features: [FeatureId]
    current_subjective_properties: {ConceptId → confidence}
    persistence_state: PersistenceState
    temporal_history: BoundedHistory<PerceivedObjectSnapshot>
```

**Key properties:**

- The agent does not store an authoritative `EntityId` guess. `EntityId` is Ground Truth bookkeeping and is inaccessible to cognition (INV-027). Continuity and identity judgments are expressed through `same_as_previous_confidence`, `continuity_confidence`, `expected_location`, and `appearance_signature`.
- The agent may merge two perceived identities into one, or split one perceived identity into two, based on observed features.
- `persistence_state` tracks whether the agent believes the object still exists, has been destroyed, has moved out of range, or has been replaced.
- Identity tracking may be wrong. The architecture must permit and propagate identity errors.

### Subjective Body Schema

The body schema is the agent's experienced model of its own body, distinct from authoritative `BodyStructure`.

**Candidate representation:**

```text
BodySchemaState:
    experienced_segments: [ExperiencedSegment]
    body_boundary: SubjectiveBoundary
    proprioceptive_state: ProprioceptiveState
    pain_signals: [PainSignal]
    balance_state: BalanceState
    capability_estimates: {ConceptId → estimated_ability}
```

**Key properties:**

- `experienced_segments` map subjective segment identities to experienced properties (position relative to self, length felt, mobility perceived). They need not correspond one-to-one with authoritative `BodySegmentId`.
- The body schema may omit segments the agent has never felt, or may include phantom segments.
- Capability estimates are subjective. An agent may believe it can jump a gap it cannot, or may believe it cannot reach something it can.
- Cognition does not receive a complete `BodyStructure` dump. It receives sensory and internal signals that update the schema incrementally.

### Self-Model

The self-model is a persistent but revisable model of the agent itself.

**Candidate representation:**

```text
SelfModel:
    autobiographical_continuity: AutobiographicalContinuity
    believed_abilities: {ConceptId → confidence}
    social_identities: [SocialIdentityAssociation]
    believed_traits: {ConceptId → confidence}
    expected_behaviour: {SituationPrototype → ActionPrototype}
    believed_reputation: {AgentId → believed_perception}
```

**Key properties:**

- The self-model may disagree with Ground Truth and with observed outcomes.
- `believed_abilities` may be inflated or deflated relative to actual capability.
- `social_identities` are associations the agent believes about its own group membership, role, or status. These are subjective and may be contested by other agents.
- The self-model is persistent memory, not active scene. Only a small relevant subset is active in any given tick.

### Predictive World Model

Agents maintain lightweight, sparse expectations about the near future.

**Candidate representation:**

```text
NearFuturePrediction:
    predicted_event: PredictedEventDescription
    predicted_time: SimulationTime
    confidence: float
    basis: PredictionBasis
    salience_if_violated: float
```

```text
PredictionError:
    violated_prediction: NearFuturePrediction
    actual_outcome: ObservedOutcomeDescription
    error_magnitude: float
    error_type: PredictionErrorType
```

**Key properties:**

- Predictions are sparse. An agent may have zero to a small handful of active predictions at any time.
- Predictions are subjective. The agent may predict things that are physically impossible, or may fail to predict things that are highly probable.
- Prediction error affects:
  - attention (surprise redirects focus);
  - salience (unexpected events become more salient);
  - memory encoding (surprising events are more likely to be stored);
  - concept revision (consistent prediction errors may revise the concept);
  - causal inference (errors reveal candidate causal links).
- Prediction is not a global physics simulator. It is pattern-based, expectation-driven, and context-limited.

### Working Memory / Active Context

Working memory holds the small set of items currently affecting cognition.

**Candidate representation:**

```text
WorkingContext:
    capacity_limit: int
    current_items: [WorkingMemoryItem]
    decay_schedule: DecaySchedule
    rehearsal_status: {MemoryId → rehearsal_count}
```

```text
WorkingMemoryItem:
    item_type: WorkingItemType
    activation_strength: float
    source: WorkingItemSource
    expected_decay_tick: SimulationTime
```

**Key properties:**

- Capacity is bounded (candidate: 4–7 items, based on cognitive science literature, but the exact number is an architecture constant).
- Items decay without rehearsal.
- Items may come from: current perception, retrieved long-term memory, active goals, active predictions, active concepts.
- Long-term memory is not in working memory unless explicitly retrieved.

### Episodic Memory Reactivation

Current perceptual patterns may reactivate stored episodic memories through partial similarity and current relevance.

**Candidate mechanism:**

```text
perceptual_feature_vector
    → similarity match against indexed episodic memory traces
    → ranked candidate list
    → relevance filtering (current goals, emotional state, context)
    → top candidates become active in working context
    → associated concepts and identities become salient
```

**Key properties:**

- Reactivation is similarity-based, not keyword-based.
- Reactivation is relevance-weighted. A memory may be highly similar but irrelevant to current goals, and therefore not reactivated.
- Reactivation may be partial. A memory may become active without the agent recognizing it as a specific past event.
- Do not create semantic events such as `RememberFatherMoment`. Reactivation is a continuous, graded process.

### Agency Attribution

Agency attribution is the learned belief that one's actions cause outcomes.

**Candidate mechanism:**

```text
attempted_action_record
    → observed_state_transition
    → temporal proximity scoring
    → contextual match scoring
    → agency_strength_update
```

**Key properties:**

- Agency attribution is learned, not innate. An agent may fail to attribute agency to itself, or may incorrectly attribute agency to itself (e.g., superstitious behaviour).
- Agency attribution feeds into:
  - self-model (believed capabilities);
  - action expectations (what will happen if I act);
  - causal inference (my action caused that effect).
- Agency attribution must be deterministic and bounded.

### Subjective Temporal Continuity

The current subjective situation is embedded in a bounded temporal envelope.

**Candidate representation:**

```text
TemporalEnvelope:
    recent_past: BoundedHistory<SubjectiveEvent>
    current_scene: SubjectiveScene
    expected_near_future: [NearFuturePrediction]
    temporal_anchors: [TemporalAnchor]
```

**Key properties:**

- `recent_past` is not full episodic memory. It is a short buffer of recent subjective events that provide context for the current scene.
- `temporal_anchors` are salient time markers (e.g., "before the storm", "since I arrived", "when the bell rang"). They help structure subjective time.
- The temporal envelope is bounded. An agent does not maintain a continuous narrative of its entire life in active context.

## Performance Constraints

1. **Bounded active state per agent.** The total active subjective state (scene + working memory + active predictions + active memories) must fit in a small, architecture-defined memory budget per agent.
2. **Sparse updates.** Do not recompute the full subjective scene for every agent every tick. Use attention-driven, event-driven, or salience-driven activation.
3. **Batch-friendly storage.** Where multiple agents share a common environment, prefer structure-of-arrays or ECS-style storage that allows batch iteration over active agents.
4. **Deterministic similarity.** Similarity matching for memory reactivation must use deterministic algorithms. No hash-map iteration ordering, no pointer-dependent hashing, no hardware entropy.
5. **No global physics inside agents.** Prediction must not invoke a full physics step. It must be pattern-based and bounded.
6. **Multi-rate support.** Different elements of the subjective scene may update at different rates (e.g., body schema updates more frequently than self-model revisions).

## Non-Goals

- Complete human brain simulation.
- Neural network emulation.
- Emotional state machine with predefined emotion enums.
- Semantic situation taxonomy (e.g., `AnxietySituation`, `CombatSituation`).
- Observer-facing narrative generation inside the cognitive system.
- LLM-based cognition.
- Real-time introspection APIs that expose the full subjective scene to external systems.
- Fully implemented Rust structs for all candidate representations. The precise data layout is a future implementation concern.

## Unresolved Questions

1. What is the minimum viable Rust type layout for `SubjectiveScene` that satisfies the bounded-size constraint while remaining extensible?
2. How does similarity matching for episodic memory reactivation scale with memory size? What indexing structure is needed?
3. How is prediction error propagated across cognitive subsystems without creating a global update cascade?
4. What is the correct balance between sparse reactivation and memory loss? How do we prevent total amnesia for inactive agents while also preventing implausible omniscience?
5. How does the subjective scene interact with the existing `AttentionState` and `SalienceState` documented in Phase 7?
6. Should `PerceivedObjectIdentity` be a typed ID (like `EntityId`) or a more complex structure? What are the determinism implications?
7. How does the body schema update when biological state changes (injury, growth, fatigue)? What signals does the biology crate send to cognition?
8. What is the minimum viable representation for a "situation prototype" that can drive both prediction and concept formation?
9. How do we test subjective scene construction without building a full agent? What unit-testable contracts can we define?
10. What is the performance budget for subjective scene reconstruction per agent per tick? A target must be set before implementation begins.

## Decision Log

- **Accepted:** The subjective scene construction layer is required before concept formation and belief implementation.
- **Accepted:** Subjective identity must be structurally distinct from authoritative identity.
- **Accepted:** Prediction error is a first-class signal that may affect attention, salience, memory, and concept revision.
- **Accepted:** Working memory is bounded and distinct from persistent autobiographical memory.
- **Accepted:** The self-model and body schema are subjective and may disagree with authoritative state.
- **Accepted:** Concrete Phase 9–10 layouts are specified by RFC-SCENE-001.

## Related Documents

- `docs/architecture/cognition-rebaseline.md` — architecture rebaseline that motivated this RFC
- `docs/architecture/invariants.md` — INV-027 through INV-035
- `docs/simulation/perceptual-features.md` — generic feature layer
- `docs/cognition/attention.md` — attention mechanisms
- `docs/cognition/memory.md` — memory structures
- `docs/cognition/prediction.md` — prediction mechanisms
- `docs/cognition/belief-inertia.md` — belief dynamics
- `docs/ontology/primitive-vs-emergent.md` — boundary between Ground Truth and subjective concepts
