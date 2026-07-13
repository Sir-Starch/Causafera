# Observer Architecture

The simulation remains headless and authoritative. The observer layer is a read-only derived view of simulation state, not a controller of it.

## Architecture Pipeline

```text
AUTHORITATIVE SIMULATION
    ↓
OBSERVER READ MODEL
    ↓
EXPLANATION ENGINE
    ↓
VERSIONED OBSERVER API
    ↓
DESKTOP APPLICATION
```

## Key Principles

- The UI never directly reads simulation internal storage
- The UI never owns authoritative state
- Observer data is derived from authoritative state through explicit transformation
- The observer layer may filter, aggregate, classify, and explain
- It may not modify, invent, or become authoritative over simulation state

## Observer Read Model

The observer read model extracts and transforms simulation state into forms suitable for human inspection. This includes:

- spatial chunk summaries
- entity snapshots
- population aggregates
- mana field samples
- causal activity indicators
- language change summaries
- concept evolution traces

## Explanation Engine

The Explanation Engine sits between the raw observer read model and the human-facing UI. It converts complex structured simulation state and provenance into understandable human explanations without modifying simulation state.

Pipeline:

```text
AUTHORITATIVE SIMULATION
    ↓
ANALYTICS / CAUSAL QUERY
    ↓
OBSERVER ANALYTICAL CLASSIFICATION
    ↓
EXPLANATION IR
    ↓
DETERMINISTIC HUMAN-READABLE RENDERING
    ↓
OPTIONAL LLM SURFACE REALIZATION
    ↓
UI
```

## Analytical Ontology

The observer layer may contain human-designed analytical categories such as:

- body-part-like structure
- periodic motion
- tremor-like motion
- disease-like pattern
- social category
- occupational category
- geographic association
- practice lineage

These are observer classifications. They are not Ground Truth domain labels exposed to agents.

## Separation of Concerns

| Layer | Can Read | Can Write | Can Classify | Can Explain |
|-------|----------|-------------|--------------|-------------|
| Simulation | Ground Truth | Ground Truth | No | No |
| Observer | Ground Truth | No | Yes | No |
| Explanation | Observer data | No | Yes | Yes |
| UI | Explanation/Observer | No | No | No |

## Invariants

- INV-012: Explanation systems are non-authoritative
- INV-013: Observer classifications cannot feed back into simulation
- INV-021: UI is an observer
- INV-022: Rendering representation is not simulation state

## Delivered Phase 26 UI

The Tauri 2 observer is the first complete protocol consumer. It receives no direct Rust runtime
references. Its current views are a bounded objective chunk projection, aggregate causal loop,
numeric timeline, chunk inspector, digest identity, and typed experiment Explanation IR. Session
buttons only select a seed and bounded scheduler progression; they do not mutate world content.

## Future UI Views

Planned observer-facing views:

- World: map and geographic overlays
- Causality: causal graph explorer
- Society: agents, relationships, organizations, social categories
- Concepts: concept origins, boundaries, and evolution
- Language: lexemes, semantic drift, language trees, word origin
- Practices: practice lineages and mutations
- Mana: field and resonance visualization
- History: timeline and state comparison
- Explanation: plain-language explanation of selected phenomena
- Performance: CPU, GPU, memory, active sets, resolution state
