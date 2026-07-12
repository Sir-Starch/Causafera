# Parcels

Parcels are the fundamental spatial units of city organization. They represent bounded regions of land with defined ownership, access rights, and physical characteristics.

## Parcel Properties

A parcel may have:

- geographic boundaries;
- elevation and slope;
- soil and substrate composition;
- hydrological characteristics;
- ownership history;
- use permissions;
- building rights;
- access routes;
- utility connections.

Phase 20 implements only a trace-backed parcel record referencing an objective spatial `PlaceId`. Boundaries remain in the spatial hierarchy; rights and ownership remain separate social claims.

## Ownership and Transfer

Ownership is not a simple boolean. It may involve:

- individual holders;
- family lineages;
- organizations;
- conditional tenures;
- shared or overlapping claims;
- disputed boundaries.

Transfers create provenance chains that may be traced historically.

## Spatial Networks

Parcel adjacency creates spatial networks that influence:

- street routing;
- utility distribution;
- fire propagation;
- social proximity;
- noise and odor transmission;
- visual access.

## Related Documents

- `docs/city/buildings.md` - Structures placed on parcels
- `docs/city/streets.md` - Access routes between parcels
- `docs/city/urban-growth.md` - How parcels are created and subdivided
- `docs/world/spatial-hierarchy.md` - Geographic hierarchy containing parcels
