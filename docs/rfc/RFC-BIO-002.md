# RFC-BIO-002: Minimal Pathogen Contracts

**Status:** Accepted

## Goal

Define the immutable authoritative boundary for pathogen lineage ancestry, property-based transmission, objective host compatibility, and causally referenced exposure without implementing disease categories, infection mutation, physiology, or epidemic simulation.

## Context

Pathogen transmission is a biological causal carrier, but Ground Truth must not contain a developer disease taxonomy or shortcut physical propagation into named routes. At acceptance time, Phase 6 causal event infrastructure was not yet implemented. The Phase 5 extension therefore established deterministic inputs that later scheduler-controlled systems could consume. RFC-TRACE-001 subsequently completed the generic provenance boundary; pathogen domain mutation remains future work.

## Authoritative primitives

### Lineage identity and ancestry

Each pathogen lineage has a `PathogenId`. An optional parent identifies direct ancestry. A registry requires parents to precede descendants, producing a connected ancestry forest with canonical deterministic traversal. Identity conveys no pathogen type, disease, symptom, or name.

### Fixed-point fractions and durations

`FractionPpm` stores a bounded fraction from zero through one million parts per million. `TickDuration` stores a strictly positive number of simulation ticks. These types avoid implicit units and floating-point behavior in authoritative pathogen properties.

### Transmission and progression properties

`PathogenProperties` records minimum infectious material dose, shedding, environmental persistence, mutation propensity, incubation duration, and infectious duration. These are property inputs, not a promise that exposure establishes infection. No airborne/contact/vector/food/water route enum exists; physical systems determine exposure through causal interactions.

### Host interaction profile

Each lineage stores zero or more `HostInteraction` records keyed by objective `PopulationLineageId`. Records contain bounded susceptibility, replication compatibility, and damage response. Profiles must be strictly ordered and unique so iteration and lookup are deterministic. These properties do not name symptoms or socially constructed population categories.

### Exposure

`PathogenExposure` records lineage identity, optional source body, target body, positive pathogen-material dose, time, and causal trace. It is evidence of a physical transmission opportunity, not an infection outcome or permission for ad hoc host mutation.

## Construction and validation

Validated constructors reject:

- fractions above one million parts per million;
- zero durations, infectious dose, shedding amount, or exposure dose;
- duplicate or non-canonically ordered host profiles;
- duplicate pathogen lineage IDs;
- missing or non-preceding parent lineages;
- mismatched parallel registry fields.

Private fields prevent authoritative values from being invalidated after construction.

## Determinism

Construction uses no randomness, floating point, hash-map traversal, locale, system time, pointer identity, or semantic labels. Registry and profile iteration order is canonical. Identical inputs produce identical values. Future mutation and transmission outcomes must use scheduler-supplied deterministic streams and stable operation ordinals.

## Mutation and provenance

This RFC defines immutable lineages, properties, and exposure records only. Infection establishment, pathogen mutation, selection, recovery, chronic state, and death must use scheduler-controlled proposal/reduce/commit phases. Every accepted change must retain causal ancestry. A `PathogenExposure` therefore requires `TraceId`, but this phase does not invent the Phase 6 provenance graph.

## Primitive and emergent boundary

Ground Truth may contain lineage ancestry, physical dose, persistence, timing, compatibility, and response magnitude. It does not contain pathogen-type enums such as bacteria or virus, route enums such as airborne or waterborne, named diseases, symptom categories, diagnoses, or cultural interpretations. Agents construct illness concepts from accessible effects and testimony.

## Performance and memory

Registry fields use deterministic structure-of-arrays storage. Variable host profiles are cold boxed slices attached to each lineage. Lookup is a deterministic linear scan in this phase. No epidemic-scale or throughput claim is made; indexing or aggregation requires benchmarks.

## Observer and explanation boundaries

No observer schema or Explanation IR is added. Future analytics may expose derived classifications with confidence and supporting trace references, but those classifications cannot feed back into simulation.

## Cross-domain effects

Geography, hydrology, ecology, migration, practice, and material movement may later create exposure dose. Physiology and immunity may determine outcomes from lineage and host properties. Agents never receive authoritative `PathogenId`, `BodyId`, `PopulationLineageId`, or `TraceId` as subjective knowledge.

## Non-goals

- Infection-course state or host physiological mutation.
- Mutation, selection, or transmission algorithms.
- Molecular or cellular biology.
- Disease, symptom, pathogen-type, or transmission-route taxonomies.
- Population aggregation, observer schemas, persistence formats, or GPU work.

## Decisions

- Treat pathogens as property-based lineages, not semantic pathogen types.
- Use integer parts per million and positive simulation-tick durations.
- Store host compatibility in sorted objective-lineage profiles.
- Represent transmission opportunity as positive physical dose plus causal trace.
- Defer all authoritative biological mutation to later provenance-aware scheduler systems.

## Unresolved future questions

- Which physical material and environmental stores carry pathogen quantities.
- How pathogen domain systems map exposure, establishment, mutation, and recovery property changes into the generic Phase 6 event proposal schema.
- Which measured workloads justify indexed host-profile lookup or aggregated epidemic state.
- How multiresolution infection aggregation preserves rare but causally important lineages.
