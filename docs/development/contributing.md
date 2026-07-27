# Contributing to Causafera

Thank you for your interest in Causafera. This project has strict architectural and philosophical requirements. Please read this document carefully before contributing.

> [`CONTRIBUTING.md`](/CONTRIBUTING.md) at the repository root is the authoritative contribution
> policy, including the contribution flow, the current CLA status, the AI-assisted contribution
> rules, and the full validation suite. [`GOVERNANCE.md`](/GOVERNANCE.md) is the authoritative
> statement of decision-making authority. This page is the documentation-tree summary of the
> architectural expectations; where the two differ, the root documents govern.

Causafera is an author-led free and open-source project. External contributions are welcome when they support the maintainer's canonical vision, but contributing does not automatically create governance or decision-making rights, and technically valid work may be declined because it does not fit the project's direction.

## Required Reading

Before making any changes, read:

1. `docs/index.md`
2. `docs/vision/project-thesis.md`
3. `docs/vision/uniqueness.md`
4. `docs/architecture/invariants.md`
5. `docs/ontology/domain-coverage-matrix.md`
6. Relevant subsystem documentation
7. Relevant ADRs
8. Relevant RFCs

## Development Workflow

- Use an ExecPlan for multi-stage work (see `PLANS.md`).
- Never introduce semantic domain enums merely for convenience.
- Never use English labels as authoritative simulation meaning.
- Never directly expose Ground Truth to agents.
- Never let LLMs mutate authoritative state.
- Never let the Explanation Engine mutate simulation state.
- Preserve deterministic RNG rules.
- Treat geography and biology as causal state.
- Preserve language intent/utterance/interpretation separation.
- Benchmark performance claims.
- Update TODO and documentation.
- Avoid unrelated opportunistic implementation.

## Code Standards

- Rust edition 2024;
- Stable Rust toolchain (pinned);
- `cargo fmt` before committing;
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` must pass;
- All tests must pass;
- No-default-features build must work.

## Codebase Navigation

Use `codebase-memory-mcp` tools for structural code queries. Prefer graph search over raw grep.

## Testing

- Add tests for new functionality;
- Include determinism tests where applicable;
- Verify observer locale independence;
- Test serialization contracts.

## Documentation

- Update relevant docs for any architectural change;
- Add ADRs for significant decisions;
- Update the Domain Coverage Matrix when adding new domains.

## Questions

Open an issue for discussion before major architectural changes.

## License and CLA

Functional software material is licensed under **AGPL-3.0-only**. Prose and non-functional explanatory documentation are licensed under **CC BY-SA 4.0**.

> **Current contribution status: preparing.** The CLA acceptance service is not yet configured or tested. Until it is, external pull requests may be prepared and discussed but cannot be merged, because CLA acceptance is a precondition for merge. Issues, discussion, and feedback are unaffected. The maintainer steps are tracked in [`docs/legal/cla-service-setup.md`](/docs/legal/cla-service-setup.md) and as `TODO-LEGAL-001` in [`docs/development/todo-backlog.md`](/docs/development/todo-backlog.md).

Before a contribution can be accepted, the contributor must separately accept the [Contributor License Agreement (CLA)](/CLA.md) through the designated CLA service. Opening a pull request does not by itself constitute acceptance. The acceptance record identifies the authenticated GitHub identity, the repository, the exact CLA revision accepted, and the timestamp; a materially changed CLA is published as a new revision and requires a new acceptance.

Only material intentionally submitted for inclusion — a pull request, a commit or patch, or another explicitly designated channel — is a contribution. Issues, bug reports, feature requests, and design discussion are not placed under the CLA.

Contributors retain their copyright. The CLA transfers no ownership. It permits proprietary and commercial outbound licensing in addition to — never instead of — the commitment that every public release containing an accepted contribution licenses it under AGPL-3.0-only for functional software material, or CC BY-SA 4.0 for prose and non-functional documentation. That is a licensing commitment, not an obligation to host or retain any file indefinitely.

## AI-Assisted Contributions

AI coding agents are explicitly allowed, including handing an open TODO to an agent. The human contributor submitting the pull request remains responsible for understanding the change, reviewing the complete diff, running and honestly reporting validation, holding the right to submit the work, correcting hallucinated or unrelated changes, and complying with the architecture, determinism, provenance, and documentation requirements. See [`CONTRIBUTING.md`](/CONTRIBUTING.md) for the full policy.

## Related Documents

- `GOVERNANCE.md` - Decision-making authority and project governance
- `CONTRIBUTING.md` - Authoritative contribution policy and validation suite
- `CLA.md` - Contributor License Agreement
- `docs/legal/cla-service-setup.md` - Maintainer checklist for enabling CLA acceptance
- `docs/development/codebase-memory.md` - Codebase knowledge graph usage
- `docs/development/changelog.md` - Changelog format
- `AGENTS.md` - Agent guidelines
- `PLANS.md` - ExecPlan format
