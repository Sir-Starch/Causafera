# User Views

The Phase 26 desktop observer delivers a bounded first set of views and preserves room for richer
domain inspection.

## Delivered Views

### World

Chart-qualified active chunks with real terrain elevation range, roughness, mana, resolution,
population, activity, and trace anchors. This is an objective observer projection, not agent
knowledge or a global planetary map.

### Causality

An aggregate physical → mana → resolution → action → population flow using current runtime counts.
It is not yet an arbitrary provenance graph explorer.

### Timeline and Inspector

A bounded client-side metric history and selected chunk details. Closing the World view stops world
queries. Timeline state is rendering state, not authoritative history.

### Explanation

Typed claims from the replay-verified bounded experiment, including evidence state, confidence,
comparison, checkpoint, and supporting trace count. The UI never invents a claim when the report is
unavailable.

## Planned Rich Views

### Society

Agents, relationships, organizations, and social categories. Shows social network structure and institutional hierarchy.

### Concepts

Concept origins, boundaries, and evolution. Displays how agents form and modify subjective categories.

### Language

Lexemes, semantic drift, language trees, and word origins. Shows linguistic change over time.

### Practices

Practice lineages and mutations. Traces how behavioral programs evolve and spread.

### Mana

Field and resonance visualization. Displays mana topology and pattern responses.

### History

Timeline and state comparison. Allows comparing simulation states at different times.

### Performance

CPU, GPU, memory, active sets, and resolution state. For development and optimization.

## View Independence

Views are independent. Opening or closing a view must not affect simulation state or other views.

## Related Documents

- `docs/ui/map-perspectives.md` - Map rendering and perspectives
- `docs/ui/language-inspection.md` - Language-specific UI features
- `docs/observer/architecture.md` - Observer layer providing view data
