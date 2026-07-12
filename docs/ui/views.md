# User Views

The future desktop application supports multiple views for inspecting different aspects of the simulation.

## Planned Views

### World

Map and geographic overlays. Shows terrain, settlements, infrastructure, and spatial phenomena.

### Causality

Causal graph explorer. Allows tracing provenance chains and understanding why phenomena exist.

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

### Explanation

Plain-language explanation of selected phenomena. Integrates with the Explanation Engine.

### Performance

CPU, GPU, memory, active sets, and resolution state. For development and optimization.

## View Independence

Views are independent. Opening or closing a view must not affect simulation state or other views.

## Related Documents

- `docs/ui/map-perspectives.md` - Map rendering and perspectives
- `docs/ui/language-inspection.md` - Language-specific UI features
- `docs/observer/architecture.md` - Observer layer providing view data
