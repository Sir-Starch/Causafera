# External Consciousness Archive

**Status:** Proposed future architecture. **Not implemented.**

## Boundary

An external consciousness archive is engine/operator infrastructure outside the ontology of every
simulated world. It is not simulated matter, mana, an in-world field, a soul in world physics, a
world location, agent memory, in-world process, or accessible afterlife. No simulated process may
reach it merely by becoming powerful, magical, intelligent, or scientifically advanced.

The archive is not observable through perception, instruments, mana, physical sensors, ordinary
observer projections, world-level Explanation queries, rituals, or a restored agent's
introspection. The engine may know it exists without making it knowledge of the world.

## Three levels of truth

| Level | Scope |
|---|---|
| World Ground Truth | Facts inside one simulation: a body died or was reconstructed, physiology resumed, an agent appeared, and witnesses perceived consequences. |
| Subjective agent models | Agent beliefs, memories, religions, theories, and interpretations, including resurrection, copying, possession, fraud, magic, or replacement. |
| Engine Meta Truth | Outside-ontology facts: archive-record identity, source/target simulation, continuation/fork decision, operator intervention, lineage, and content hashes. |

Engine Meta Truth must not automatically leak into World Ground Truth or agent knowledge. In
particular, ordinary observer and Explanation interfaces remain world-facing and must not disclose
archive metadata.

## Capture and identity

At a future death boundary, the archive records the last fully committed cognitive state:

last completed cognitive transition → lethal or terminal world event → terminal consciousness
capture → agent becomes non-living → immutable archive record available to operator infrastructure

The archive does not continue updating cognition after death; a separate afterlife simulation
would require separate design. It stores what the simulated cognitive model needs to reproduce
future behavior at its supported abstraction, not biological neurons or a claim about real
consciousness.

Future modeling must distinguish, without redesigning current identifiers:

- identity lineage: engine-level continuity or ancestry;
- incarnation: one embodied lifetime or restoration episode;
- runtime entity: one currently scheduled object in one simulation.

Names such as IdentityId, IncarnationId, and RuntimeEntityId are illustrative only. The archive
must not make cognitive state permanently inseparable from a runtime entity.

## Restoration, copying, and transfer

The engine may restore an archived state in two Meta-Truth modes.

- **Continue lineage:** the new incarnation continues an existing engine identity lineage.
- **Fork lineage:** the same archive record starts a new engine identity lineage.

Either creates a new incarnation and runtime entity. At restoration, both can have identical
memories, beliefs, skills, goals, dispositions, emotional state, autobiographical identity,
social models, and last remembered perceptions. Cognition must not receive a copy/original/forked
flag, archive-record ID, source identity, or continuation-mode flag. They are therefore internally
indistinguishable unless an operator introduces an ordinary observable difference.

Multiple branches may instantiate from one record, then diverge by experience. Each may sincerely
identify with the archived past while the engine records separate lineages.

A record from simulation A may later be instantiated in simulation B as either a continuing or
forked lineage. Example: a person from world A is captured at death and instantiated in a
biologically compatible body in world B. They remember world A, but nothing in B proves that A
exists, that they continued, or that they are a copy.

## Portable representation and embodiment

Portable cognitive content may include memories, beliefs with confidence, concepts, goals,
emotional dispositions, learned strategies and abstract skills, autobiographical identity,
subjective models of people/places/institutions/events, unresolved commitments, and the last
committed cognitive state.

It must exclude live source references: runtime IDs, coordinates, scheduler state, object or
ownership handles, capability flags bypassing target physics, direct pointers, and component
storage references. Memories of source entities remain autobiographical content, never live
engine references.

The target embodiment obeys target-world laws. Sword knowledge may transfer while coordination
must adapt to the body; magic knowledge may transfer while capability is decided by target mana
and physical laws; source-language knowledge may transfer while target-language knowledge needs a
causal mechanism. Source capability flags must not be imported.

## Archive records, authoritative entry, and reproducibility

Records should be immutable and content-addressed. A future record may identify its record,
source world, lineage/incarnation/runtime entity, terminal tick, terminal cognitive state,
death-event reference, schema version, and content hash. This is conceptual, not a final schema,
storage engine, binary format, or migration system.

Restoration enters through explicit external intervention and normal authoritative mutation:

RestoreConsciousnessRequest → validation → body creation/reconstruction proposal → engine-level
continuation/fork decision → cognitive-state loading → scheduler → authoritative commit →
provenance → ordinary physical consequences

It must never be an invisible mutation such as setting dead to false. World observers see physical
and behavioral consequences, not archive truth.

Replay and save/resume must not consult mutable external archive contents. Possible future
approaches include immutable content-addressed records, sealed intervention payloads,
operator-level logs containing exact hashes, versioned restoration metadata, or including the
required immutable cognitive payload in a replay bundle. A replay must never silently restore a
different consciousness because an external file changed.

## Compatibility constraints

This proposal preserves deterministic scheduling, proposal/commit mutation, persistence,
deterministic replay, and causal provenance. Engine Meta Truth remains inaccessible to agents,
mana, world observers, and world-level Explanation. Current product code must not depend on this
archive, and current identifiers/domain abstractions must not be redesigned solely for it.

## Current status

No external consciousness archive exists. Resurrection, copying, terminal consciousness
serialization, identity-lineage infrastructure, and cross-simulation consciousness transfer are
not implemented. The completed actor/material/mana vertical slice does not require or establish
them; existing snapshots and agent state are not evidence that these capabilities exist.

Related existing research deliberately leaves concrete death, cognitive, and transfer adapters
deferred: [RFC-META-001](../rfc/RFC-META-001.md) and
[RFC-ISEKAI-001](../rfc/RFC-ISEKAI-001.md).
