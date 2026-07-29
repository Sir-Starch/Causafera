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
OPTIONAL LLM SURFACE REALIZATION (TERMINAL GATE; NOT SCHEDULED)
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

## Bounded bootstrap read model

The runtime summary carries a bounded projection of the canonical production bootstrap record:
the opaque plan ID, world seed, stage count, validation status, the configured population and
promotion bounds, and at most six receipts — each with its stage, completion time, result
fingerprint, completion trace, and dependency trace anchors.

It is a read model in the strict sense. It carries no runtime handle, no authoritative actor or
place identity, no stage targets, and no rendered process name; the opaque process schema IDs are
deliberately absent from it, because a reader that could see them would be one step from naming
them. Observer polling cadence, locale, and query order do not alter authoritative state or digests.

The wire fields are additive — 28 onwards on the existing runtime summary — so `OBSERVER_PROTOCOL_V1`
is unchanged and fields 1..=27 keep the meaning they had. A payload written before the summary
existed decodes with schema version 0, which means "no bootstrap evidence in this payload" rather
than "an empty record". Both the Rust and TypeScript decoders bound the receipt and dependency lists
before growing them and reject a non-canonical order.

Explanation exposes two typed claims: schema 18 reports how many stages the plan declares against
how many receipts closed them, anchored to the completion traces; schema 19 reports the bounded
canonical window they span. An incomplete or unevidenced record answers with the existing unknown
state at zero confidence rather than erroring. Neither claim reports a fingerprint as a numeric
value — a fingerprint is an equality identity, not a magnitude.

## Detailed Development cadence

Observer development follows simulation and Explanation evidence needs. The immediate priority is
bounded causal event slices, typed domain time series and state deltas, bootstrap receipts,
resolution transitions, and explicitly separated objective/subjective projections. These read
models enable capability validation and causal explanation without direct storage access.

The desktop UI is updated in coherent milestone batches after read-model contracts stabilize. A
new internal simulation field does not automatically require a UI component. Diagnostic protocol
coverage may advance without immediate visual polish.

The optional LLM step shown in the eventual pipeline is a terminal, unnumbered gate and is not
current observer or UI work.

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
