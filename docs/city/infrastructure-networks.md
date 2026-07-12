# Infrastructure Networks

Cities contain multiple overlapping infrastructure networks that distribute resources, remove waste, and enable communication.

## Network Types

Plan support for:

- water supply - sources, treatment, distribution, pressure;
- sewage - collection, transport, disposal;
- waste - removal, processing, reuse or disposal;
- roads and bridges - surface transport;
- ports and harbors - waterborne transport;
- energy - if applicable to the technological level;
- communication - if applicable (signal towers, couriers, etc.).

## Implemented foundation

Phase 20 implements bounded directed `InfrastructureNetwork`, `InfrastructureNode`, and `InfrastructureLink` records. Network purpose is an opaque historically assigned schema, not a `Road`/`Water`/`Sewage` enum. Nodes retain place, capacity, condition, and trace; links retain endpoints, capacity, length, condition, material-lot provenance, and trace. Flow physics, failures, maintenance, and social administration remain deferred.

## Network Properties

Infrastructure networks have:

- physical topology (nodes and connections);
- capacity and flow rates;
- maintenance state;
- construction history and material provenance;
- ownership and administrative responsibility;
- failure modes and cascade effects.

## Emergent Patterns

Spatial networks may create unintended large-scale informational patterns.

**Example:**

```text
repeated sewer junction geometry
↓
city-scale spatial recurrence
↓
mana field response
```

Urban planning may become an unintentionally magical activity.

## Related Documents

- `docs/city/streets.md` - Surface transport network
- `docs/city/maintenance.md` - Infrastructure upkeep
- `docs/city/fire.md` - Fire and water network interactions
- `docs/world/hydrology.md` - Water systems and geography
