# User Views

The desktop observer organises its delivered capability into five areas and preserves room for
richer domain inspection. Areas are the top-level navigation; an inspector dock carries the
selection context for whichever area is open.

## Delivered Areas

### Observatory

The primary surface. Run identity (seed, negotiated protocol, tick, digest anchors), the instrument
cluster, and the three quantities that move in a live session: the mana field as total and peak cell
intensity, causal accretion as committed traces, and actor action admission as the committed share
of attempted actions. Carries a chart strip and a recent-transition preview that lead into the other
areas, plus a summary of instrument coverage.

### Chart

The spatial instrument. A pannable, zoomable map of one chart's chunk lattice, inspected through
analytical lenses: relief, elevation range, roughness, material surface, mana field, population,
causal activity and causal resolution are drawn from real observer data; interpolated isolines and
neighbour-difference vectors are observer-side preview constructions and are marked as such; agents,
knowledge, language, social structure, practices, economy and ecology are listed as awaiting a read
model and say what they are waiting for.

Detail follows scale: a field of colour when chunks are small, chunk glyphs and values at reading
size, and the 32³ cell lattice with marks at real cell positions when zoomed in. Ground beyond the
received extent is hatched as unsurveyed. This is an objective observer projection, not agent
knowledge and not a global planetary map; adjacency is coordinate ordering, not measured distance.

The chart profile and the chunk register sit beneath the map as supporting reads of the same
selection, and the inspector decodes the material surface cell ordinal into its position inside the
32³ chunk lattice.

### Flux

Causal activity over run time. Rate recorders over the observer-side summary series, the surface
condition ladder — one step function per tracked material surface, with rings for contact traces,
diamonds for mana physical effects, and squares for local mana coupling — and the bounded transition
ledger. Selecting a trace anchor anywhere filters the ledger. That is the full extent of provenance
navigation the current protocol supports, and the interface says so rather than implying an ancestry
graph it cannot draw.

Local mana gate transitions are shown with an explicit empty state: only transitions into the closed
state are projected, so an empty list is an observation rather than a gap.

### Assay

Typed claims from the replay-verified bounded experiment, each with evidence state, confidence,
comparison context, and its trace anchors, together on the same card (INV-026). Claim schemas carry
presentation reading notes; an unregistered schema renders generically rather than disappearing. A
claim marked `Unknown` is presented as a result — absence of evidence is not negative evidence — and
an assay that describes an earlier tick is marked stale rather than silently redisplayed.

### Instrument

Protocol negotiation, the run bounds of the current configuration, a log of real exchanges with
measured byte counts and durations, and the coverage register: every observable the project has
defined, with its state (read, bounded, no projection, domain not ready) and its domain and observer
maturity.

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

- `docs/ui/observer-application.md` - Frontend architecture and design system
- `docs/ui/observer-projection-gaps.md` - Projections the frontend is waiting on
- `docs/ui/map-lenses.md` - The map's lens contract and how to extend it
- `docs/ui/map-perspectives.md` - Map rendering and perspectives
- `docs/ui/language-inspection.md` - Language-specific UI features
- `docs/observer/architecture.md` - Observer layer providing view data
