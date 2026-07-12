# RFC-PRACTICE-001: Evolvable Practice Representation

**Status:** Accepted

## Summary

Practices are bounded structured behavioural programs with explicit lineage. They are separate from agent explanations, observer glosses, skills, and authoritative state mutation.

## Decision

A practice lineage contains an opaque `PracticeId`, an optional parent, and at most 64 structural instructions. Instructions may propose an opaque learned action pattern, wait, test numeric subjective evidence, branch, repeat a bounded earlier span, or halt. Action and condition identities carry no engine-defined meaning.

Construction rejects empty or oversized programs, zero-duration operations, invalid targets, and unbounded repeats. Execution has fixed step and emission budgets. Evidence is canonically ordered and uses only agent-local condition identities and integer values.

Execution produces a `PracticeExecution` record containing timing and action proposals. It does not mutate bodies, materials, places, or scheduler state. Those proposals require later phase-controlled validation and commit.

Mutation replaces one instruction and creates a child whose parent is the source lineage. The representation therefore supports inspectable drift without inventing a semantic transformation taxonomy.

## Consequences

- Identical program, evidence, and time inputs produce identical execution records.
- Named rituals, crafts, skills, professions, and goals remain emergent.
- Opaque actions can later connect to motor/action systems without giving practices Ground Truth identity.
- Full imitation fidelity, resource constraints, coordination, diffusion, and institutional embedding remain future work.

## Rejected alternatives

- English action strings: they make UI vocabulary authoritative.
- Semantic operation enums: they predefine the practices intended to emerge.
- Closures or arbitrary scripts: they weaken validation, persistence, determinism, and inspection.
- Direct execution mutation: it violates scheduler-controlled proposal/commit boundaries.
