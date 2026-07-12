# Trust

Trust is the willingness to rely on another agent's information or actions. It shapes belief formation, social structure, and information flow.

## Trust Representation

```text
TrustState:
    trust_relationships: {AgentId → TrustRelationship}
    default_trust: float
    trust_history: [TrustEvent]
    reputation_state: ReputationState
```

### Trust Relationship

```text
TrustRelationship:
    target_agent: AgentId
    competence_trust: float
    benevolence_trust: float
    integrity_trust: float
    general_trust: float
    interaction_history: [InteractionRecord]
    betrayal_count: int
    cooperation_count: int
```

## Trust Dimensions

Trust has multiple dimensions:

- **Competence**: belief in ability
- **Benevolence**: belief in good intentions
- **Integrity**: belief in honesty and consistency
- **Predictability**: belief in reliability

## Trust Formation

Trust forms through:

- **Direct experience**: personal interaction history
- **Indirect evidence**: observation of others' interactions
- **Reputation**: social reports about target
- **Category**: trust based on group membership
- **Institution**: trust based on role or position
- **Coercion**: trust based on power imbalance

## Trust and Belief

Trust affects belief formation:

- **Source trust**: trusted sources' claims are more believed
- **Prestige bias**: high-status sources are more believed
- **Authority bias**: authoritative sources are more believed
- **Confirmation**: trusted sources' claims confirm existing beliefs

## Trust and Social Structure

Trust shapes social structure:

- **Cooperation**: trust enables cooperation
- **Exchange**: trust enables trade
- **Organization**: trust enables organizations
- **Hierarchy**: differential trust creates hierarchy
- **Exclusion**: lack of trust creates boundaries

## Trust and Information

Trust affects information flow:

- **Transmission**: trusted agents' information spreads
- **Filtering**: untrusted agents' information is ignored
- **Distortion**: trusted agents may mislead
- **Cascades**: trust chains create information cascades

## Trust Degradation

Trust degrades through:

- **Betrayal**: violation of trust
- **Incompetence**: repeated failure
- **Deception**: discovery of dishonesty
- **Inconsistency**: unpredictable behavior
- **Rumors**: negative social reports

## Determinism

Trust processes must be deterministic given:

- current trust state
- interaction history
- social context
- personality parameters
- biological state

## Performance

Trust computation may be frequent. Strategies:

- Cached trust values for stable relationships
- Event-driven updates for significant interactions
- Aggregate representation for group trust

## Related Documents

- `attention.md` — attention to trusted sources
- `memory.md` — memory stores interaction history
- `belief-inertia.md` — trust reinforces belief inertia
- `goals.md` — trust affects goal adoption
- `strategic-communication.md` — trust affects communication interpretation

## TODO Categories

- `COG` — cognition
- `BELIEF` — belief systems
- `SOCIAL` — social systems

## Phase 12 Implementation Status

The minimal `TrustStore` uses at most 32 opaque `SubjectiveSourceId` hypotheses rather than authoritative `AgentId` values. Trust is a deterministic fixed-point running mean of observed correspondence and weights incoming belief evidence. Competence dimensions, reputation, indirect reports, categories, institutions, deception models, and social-network propagation remain future work.
