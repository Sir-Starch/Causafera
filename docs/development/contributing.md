# Contributing to Causafera

Thank you for your interest in Causafera. This project has strict architectural and philosophical requirements. Please read this document carefully before contributing.

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

All source code is licensed under **AGPL-3.0-only**. All documentation is licensed under **CC BY-SA 4.0**.

> **Current contribution status:** Causafera is a personal hobby and research project. External code contributions are not currently accepted. They may be accepted after the CLA acceptance workflow is configured. Issues, discussion, and feedback remain welcome.

Before a contribution can be accepted, the contributor must separately accept the [Contributor License Agreement (CLA)](/CLA.md) through the designated CLA service or electronic-signature process. Opening a pull request does not by itself constitute acceptance. The acceptance record must identify the contributor's verified identity, CLA version, timestamp, and an associated pull request or commit.

The CLA permits proprietary and commercial outbound licensing while requiring accepted source contributions to remain available in the public project under AGPL-3.0-only and accepted documentation or other non-code contributions under CC BY-SA 4.0.

## Related Documents

- `docs/development/codebase-memory.md` - Codebase knowledge graph usage
- `docs/development/changelog.md` - Changelog format
- `AGENTS.md` - Agent guidelines
- `PLANS.md` - ExecPlan format
