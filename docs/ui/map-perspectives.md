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

## Rendering

The bounded Phase 26 view renders at most the active-chunk envelope as semantic controls. This is
appropriate for the demonstrated three-chunk workload and preserves keyboard inspection.

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
