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
