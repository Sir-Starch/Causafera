# Preserved Maturity-Audit Groundwork

## Status

Historical, non-blocking groundwork from the indefinitely paused frozen-baseline maturity audit.
It is not a maturity claim about current Causafera and does not gate current implementation.

## Frozen evidence boundary

The work recorded here applies only to source baseline
`26026fb3862e8d178a2e59df7a68a2901e80b123`, tree
`8507defcd090b107eaf695b1289bd42d1ebd2f32`, and run
`audit-26026fb3862e-20260715T004000Z`. Counts, mappings, and audit conclusions below are
historical evidence for that baseline only. They must not be presented as claims about the current
HEAD or used to infer current capability maturity.

## Completed Todos 1–4

1. **Frozen run identity.** Todo 1 captured the source baseline, tree identity, clean audit
   worktree, and graph-index status.
2. **Methodology.** Todo 2 activated the native audit plan, with baseline-bound source blobs,
   capability-level evidence, cumulative maturity semantics, and the distinction between shallow
   inventory and four fixed deep-audit families.
3. **Checker and fixtures.** Todo 3 created the zero-dependency checker, closed schemas, adapter
   contracts, valid examples, and negative fixtures under
   [`tools/audit/`](https://github.com/Sir-Starch/Causafera/tree/main/tools/audit).
4. **Shallow inventory.** Todo 4 reconciled the 30 domain rows against **132 baseline sources**,
   **105 complete and 27 explicitly incomplete LSP captures**, and **61 capability/evidence rows**.
   These are baseline-specific inventory counts, not current-Causafera coverage or maturity
   measurements.

## Reusable rules

- A crate, type, test name, or documentation page does not prove implementation.
- Fixtures and demo construction do not prove production reachability.
- Coupling requires explicit physical or informational carriers, not a shared label or adjacent
  crate.
- Every significant authoritative change requires committed causal provenance.
- Where applicable, accepted behaviour requires replay, persistence/save-resume, bounded read-only
  observation, deterministic Explanation, and explicit negative controls.

## What was not completed

The four deep audits, canonical maturity result, portable evidence bundle, old sequencing graph,
and exhaustive M0–M5 classification were not completed. They are not prerequisites for current
development. The audit remains paused indefinitely; its tooling is historical, non-blocking, and
may be reused only on explicit demand with a fresh current-HEAD evidence boundary.

## Links and scope

- [Paused maturity-audit plan](https://github.com/Sir-Starch/Causafera/blob/main/plans/detailed-development-maturity-audit.md)
- [Completed actor/material/mana implementation record](https://github.com/Sir-Starch/Causafera/blob/main/plans/actor-material-mana-loop.md)
- [Capability catalog](https://github.com/Sir-Starch/Causafera/blob/main/tools/audit/capability-catalog.json)
- [Checker schema contracts](https://github.com/Sir-Starch/Causafera/blob/main/tools/audit/schema-contracts.json)
- [Adapter contracts](https://github.com/Sir-Starch/Causafera/blob/main/tools/audit/adapter-contracts.json)
- [Fixture manifest](https://github.com/Sir-Starch/Causafera/blob/main/tools/audit/fixture-manifest.json)
- [Todo 4 inventory builder](https://github.com/Sir-Starch/Causafera/blob/main/tools/audit/build-task4-inventory.mjs)

The audit-specific [`validate-capability-audit.mjs`](https://github.com/Sir-Starch/Causafera/blob/main/tools/audit/validate-capability-audit.mjs)
remains frozen-audit tooling. It must not be weakened or used as current general plan validation:
it deliberately expects the old audit to be active and baseline-bound. Current plans are reviewed
against `PLANS.md`, their native ExecPlan headings, current-HEAD source evidence, focused
documentation/link checks, `git diff --check`, and implementation-time targeted tests.
