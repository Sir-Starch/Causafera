# Historical Bootstrap

The main observed city must not start as a historically empty procedural settlement. It must have a history that shaped its current state.

## Bootstrap Strategy

Use a hybrid historical initialization strategy:

### Deep History

Causally constrained historical synthesis at low resolution. This phase simulates:

- geological formation
- climate establishment
- ecological succession
- early human migration
- language divergence
- initial settlement patterns
- early technological development

Deep history operates at low causal resolution. Individual agents are not simulated. Population-level processes drive change.

### Recent History

Accelerated simulation at higher causal resolution. This phase simulates:

- specific settlement founding
- building construction and decay
- family lineages
- language change and semantic drift
- practice development and transmission
- institutional formation
- economic specialization
- political development
- infrastructure accumulation

Recent history operates at higher resolution for the observation focus area. Distant regions remain at lower resolution.

## Bootstrap Outputs

The historical bootstrap must produce:

- **Old districts**: neighborhoods of different ages and characters
- **Buildings of different ages**: structures from different construction periods
- **Families**: household structures with genealogical depth
- **Language history**: established languages with phonologies, lexicons, and grammatical structures
- **Semantic drift**: words with changed meanings over time
- **Inherited practices**: techniques and customs passed down through generations
- **Abandoned infrastructure**: ruins, closed roads, filled canals
- **Ruins**: remains of older settlements or structures
- **Legal precedent**: accumulated decisions and customary law
- **Institutional memory**: organizations with historical continuity
- **Local material use traditions**: preferred building materials and techniques

## Bootstrap Provenance

Historical bootstrap requires provenance. Every feature of the initialized world must be traceable to its historical cause.

Do not invent lore prose as authoritative history. The bootstrap must generate causal histories, not narrative descriptions.

## Bootstrap and Causal Resolution

The Causal Resolution Field applies during bootstrap:

- High resolution for the observation focus area
- Lower resolution for distant regions
- Variable resolution for historically significant events

## Determinism

Historical bootstrap must be deterministic given:

- world_seed
- bootstrap parameters
- resolution parameters

## Implemented orchestration foundation

Phase 21 implements `HistoricalBootstrapPlan` as a bounded canonical DAG of opaque synthesis processes. Each stage declares a simulation-time interval, numeric detail ordinal, target chunks, prior stages, external causal traces, and a parameter fingerprint. Stable per-stage seed contributions depend only on explicit numeric inputs.

Concrete domain adapters still perform authoritative changes through READ → PROPOSE → REDUCE → COMMIT. After commit they return a receipt containing the result fingerprint and committed trace. A complete bootstrap record is accepted only when every planned stage has one receipt whose causes exactly continue the declared dependency traces.

This foundation does not generate the output examples above. Old districts, families, language depth, institutions, and ruins remain targets for later domain synthesis; neither their existence nor their plausibility is claimed by Phase 21.

## Implemented production bootstrap record

The runtime's production bootstrap now executes that canonical contract rather than a parallel
model of its own. `causafera-runtime::RuntimeBootstrapRecipe` is the executable adapter around one
`causafera_world::HistoricalBootstrapPlan`; there is no second plan type.

The plan the runtime builds today declares exactly six stages, in this fixed dependency chain:

| Stage | Process schema | Canonical span | What the adapter commits |
| --- | --- | --- | --- |
| 1 | `0x0B01` | `[0, 1]` | terrain generation per active chunk |
| 2 | `0x0B02` | `[1, 2]` | one material surface per active chunk |
| 3 | `0x0B03` | `[2, 3]` | the population aggregate |
| 4 | `0x0B04` | `[3, 4]` | actor promotion out of that aggregate |
| 5 | `0x0B05` | `[4, 5]` | material activity on the aggregate |
| 6 | `0x0B06` | `[5, 6]` | thermal fields and reservoirs |

Process schema IDs are opaque numeric identities. They are not names, and nothing downstream may
translate them into one.

The canonical spans are a bootstrap **ordering** timeline. They do not advance
`RuntimeState::advanced_through`, and every stage effect keeps the existing Lifecycle timestamp
convention at simulation time zero.

Plan identity is content-addressed: it is derived from the world seed and every stage's process,
span, detail ordinal, targets, dependencies, external causes, and parameter fingerprint. Targets are
sorted `ChunkId` values derived from the active `ChartChunkCoord` set by a domain-separated
addressing function — identity only, never a distance, an extent, or an ownership claim.

### One terminal receipt per stage

After a stage's adapter runs, the coordinator reads back **what the trace store actually recorded**
during that stage rather than what the adapter reported, computes a result fingerprint from the
canonical projection of those committed effects, and commits one completion event. That event's
effect is the authoritative transition of the stage's bounded result state from an absent sentinel
to the result fingerprint; its causes are the previous stage's receipt plus every effect trace the
stage committed. The receipt's own causes are exactly the dependency ancestry
`HistoricalBootstrapPlan::validate_receipts` requires.

A stage with no domain effect — an empty population stage, no promotions, no material activity —
still commits that transition, so it still has a receipt anchored to a real committed effect. The
record is validated through the canonical contract before the constructed runtime is returned.

### Persistence, digest, and observer

- `SECTION_POPULATION_BOOTSTRAP` carries the complete plan, the bounded per-stage result state, and
  the receipts at section major 2. Major 1 carried neither a plan nor a result and fails closed.
- `CURRENT_DIGEST_SCHEMA_VERSION` is 7: the record is authoritative `physical_state_digest` input.
- Import re-derives the plan from the persisted configuration and requires it to match, then checks
  every completion trace, effect, result, and cause against the persisted trace store. At bootstrap
  time it also requires the configured population to be conserved across aggregates and promoted
  actors, and every promoted actor's ancestry to be a trace the actor-promotion receipt named.
- The observer receives a bounded read-only summary of at most six receipts on the existing runtime
  summary, and two typed Explanation claims (schemas 18 and 19) for completeness and canonical
  window. Neither renders a process name.

### What this is not

Six stages is the complete implementation surface today. It is a bound on what the runtime executes,
not a claim that historical synthesis is only ever six steps and not evidence of deep history. No
geology, climate, ecology, language, settlement, institution, or economy synthesis is implemented,
and none of the Bootstrap Outputs above is produced.

## Performance

Historical bootstrap may be computationally expensive. Strategies:

- Low resolution for deep history
- Accelerated time for early phases
- GPU acceleration for population-level processes
- Cached intermediate states

## Related Documents

- `geography-philosophy.md` — geographic causality
- `settlements.md` — settlement formation
- `world-generation-provenance.md` — provenance tracking
- `docs/language/language-bootstrap.md` — language initialization
- `docs/simulation/technology-and-invention.md` — technological development

## TODO Categories

- `WORLD` — general world systems
- `TRACE` — provenance and causal tracing
