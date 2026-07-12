# Isekai Architecture

The isekai subsystem lives in the `ontopolis-isekai` crate. It models cross-world transfer: the physical or metaphysical processes by which agents, objects, information, or patterns move between Ontopolis and other worlds (including Earth).

## Core Principle

Cross-world transfer must be a physical or metaphysical process, not a narrative convenience. The subsystem must model what actually happens when something crosses between worlds, what properties persist, what is lost, and how the transferred entity interacts with Ontopolis causality.

## Phase 22 Transfer Boundary

A transfer is represented by `CrossWorldTransferPlan` and a committed receipt with:

- A source world and location
- A target world and location
- An opaque mechanism schema rather than a semantic transfer enum
- A transferred entity or pattern
- Properties that persist across the transfer
- Properties that are lost or transformed
- Canonical payload/property fingerprints and exact causal ancestry

The plan cannot mutate state. Concrete adapters use the normal proposal/reduce/commit boundary. Mechanism schemas preserve metaphysical openness: the core contract does not decide whether a crossing is transport, copying, binding, reincarnation, or something else.

## What Isekai Is Not

Isekai is not:

- A character creation screen
- A technology unlock system
- A source of predefined heroes with fixed abilities
- A narrative device without physical consequences

Transferred entities are physical presences in Ontopolis. They occupy space, consume resources, produce waste, and participate in causal chains.

## Interaction with Other Domains

- **Metaphysics**: What is identity persistence across worlds? What happens to consciousness?
- **Cognition**: Transferred agents bring foreign conceptual priors
- **Language**: Transferred agents may speak languages unknown in Ontopolis
- **Epistemics**: Transferred agents may possess knowledge without supporting infrastructure
- **Mana**: Transferred patterns may have unexpected mana interactions
- **Society**: Transferred agents must be integrated, excluded, or exploited

## Related Documents

- `docs/isekai/transfer-types.md` - Kinds of cross-world transfer
- `docs/isekai/foreign-memory.md` - What transferred agents remember
- `docs/isekai/imported-priors.md` - What foreign agents bring with them
- `docs/isekai/translation-impact.md` - How foreign concepts affect local language
- `docs/isekai/historical-arrivals.md` - The history of cross-world transfer
- `docs/isekai/causal-contamination.md` - How foreign causality affects local systems
- `docs/metaphysics/cross-world-continuity.md` - Identity across worlds
