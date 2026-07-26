# Map Perspectives

The UI supports multiple map perspectives that reflect different knowledge states.

## Perspective Types

### Ground Truth Map

Shows actual geographic state. Available to developers and for analytical purposes. Not available to simulated agents.

Phase 26 delivers only a bounded objective active-chunk projection for this perspective. It carries
chart identity and local 3D chunk coordinates, and must not be interpreted as a seamless global
Cartesian map.

### Agent Known Map

Shows what a selected agent knows about geography. May be incomplete, incorrect, or differently categorized.

### Organization Known Map

Shows the geographic knowledge shared within an organization. May differ from individual agent knowledge.

### Historical Map

Shows geographic state at a selected past time. Supports comparing historical and current geography.

## Map Properties

Maps may be wrong. Examples:

- A trader may know a route but not exact terrain.
- A government may claim a border it does not control.
- An agent may misidentify a geographic feature type.
- Different agents may use different naming conventions.

The agent-known, organization-known and historical perspectives are present in the observer as
`awaiting` lenses: each is listed in the lens catalogue and states the read model it needs, rather
than being omitted or simulated. See `docs/ui/map-lenses.md`.

## Rendering

The Ground Truth perspective is rendered by the chart instrument: a canvas plan view of one chart's
chunk lattice with viewport culling and three levels of detail, driven by the lens contract. It is
written for a chart far larger than the demonstrated three-chunk workload — a screenful costs the
same at any chart size — and keyboard inspection is preserved through the chunk register and the
map's own arrow, zoom and reframe keys.

Future large maps use WebGPU for:

- terrain and elevation;
- spatial fields (mana, climate);
- large point datasets (population, activity);
- dense overlays (infrastructure, political boundaries).

Do not render one DOM element per resident.

## Related Documents

- `docs/ui/views.md` - View system including World view
- `docs/world/spatial-hierarchy.md` - Geographic hierarchy
- `docs/observer/architecture.md` - Observer providing map data
