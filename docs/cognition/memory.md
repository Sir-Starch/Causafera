# Memory

Memory is the storage and retrieval of information. It is limited, fallible, and subject to distortion.

## Memory Representation

```text
MemoryState:
    working_memory: WorkingMemory
    long_term_memory: LongTermMemory
    retrieval_cues: [RetrievalCue]
    memory_confidence: float
    memory_availability: float
```

### Working Memory

```text
WorkingMemory:
    capacity: int
    current_contents: [MemoryItem]
    decay_rate: float
    rehearsal_status: [RehearsalState]
```

Working memory holds:
- current perceptual input
- active goals
- recent actions
- retrieved long-term memories

Working memory is small (candidate: 4-7 items) and decays quickly without rehearsal.

### Long-Term Memory

```text
LongTermMemory:
    episodic_memories: [EpisodicMemory]
    semantic_memories: [SemanticMemory]
    procedural_memories: [ProceduralMemory]
    associative_network: AssociativeNetwork
    consolidation_status: [ConsolidationState]
```

## Memory Types

### Episodic Memory

Episodic memory stores specific events:

```text
EpisodicMemory:
    event_time: Time
    event_location: WorldCoord
    participants: [AgentId]
    actions: [ActionDescription]
    outcomes: [OutcomeDescription]
    emotional_tag: EmotionalTag
    rehearsal_count: int
```

### Semantic Memory

Semantic memory stores general knowledge:

```text
SemanticMemory:
    concept_id: ConceptId
    associated_properties: {Property → confidence}
    source_provenance: [SourceRecord]
    confidence: float
```

### Procedural Memory

Procedural memory stores skills and habits:

```text
ProceduralMemory:
    practice_id: PracticeId
    execution_quality: float
    automaticity: float
    error_patterns: [ErrorPattern]
```

## Memory Processes

### Encoding

Information enters memory through:
- attention (attended information is more likely encoded)
- emotional arousal (emotional events are better remembered)
- rehearsal (repeated information is better remembered)
- elaboration (richly processed information is better remembered)

### Storage

Stored information:
- decays over time
- may be consolidated (stabilized)
- may be reorganized during sleep or rest
- may be lost due to interference

### Retrieval

Retrieval depends on:
- cues (context, emotion, associations)
- recency (recent memories are more accessible)
- frequency (frequently accessed memories are more accessible)
- emotional state (mood-congruent retrieval)

Retrieval may fail:
- forgetting (memory no longer accessible)
- blocking (temporary inability to retrieve)
- distortion (retrieval alters memory)

## Memory Distortion

Memories are not perfect records:

- **Forgetting**: loss of information over time
- **Confabulation**: filling gaps with plausible but false information
- **Source confusion**: misremembering where information came from
- **Suggestibility**: incorporating post-event information
- **Bias**: remembering in ways consistent with current beliefs

## Memory and Other Domains

Memory interacts with:

- **Perception**: memory guides perceptual interpretation
- **Attention**: attention determines what enters memory
- **Belief**: memory provides evidence for beliefs
- **Decision**: memory provides options and outcomes
- **Language**: memory stores vocabulary and grammar
- **Identity**: memory creates personal narrative

## Determinism

Memory processes must be deterministic given:

- current memory state
- encoding events
- retrieval cues
- time elapsed
- biological state

## Performance

Memory data may be large. Strategies:

- Sparse representation for distant or inactive agents
- Event-driven updates for significant memories
- Aggregate representation for common knowledge
- Compression for old or weak memories

## Related Documents

- `attention.md` — attention determines encoding
- `salience.md` — salient information is better remembered
- `prediction.md` — memory supports prediction
- `belief-inertia.md` — memory supports belief maintenance
- `goals.md` — memory stores goal history

## TODO Categories

- `COG` — cognition
- `MEM` — memory

## Phase 10 Implementation Status

`WorkingContext` is implemented as at most eight active items with deterministic fixed-point ranking, decay, and rehearsal. It is structurally separate from `EpisodicStore`, a capped minimal cold store used to validate the boundary rather than define the final persistence format.

Episodic reactivation compares quantized appearance signatures, multiplies similarity by numeric relevance and stored strength, and activates at most four ranked episodes. It uses no keyword, event-name, concept enum, Ground Truth ID, or trace reference. Rich long-term memory, distortion, consolidation, and durable indexing remain future work.
