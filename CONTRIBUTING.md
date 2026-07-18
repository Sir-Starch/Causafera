# Contributing to Causafera

Causafera is an experimental pre-alpha research and hobby project with strict causal, deterministic,
and provenance requirements.

> **Current contribution status:** external code and documentation contributions are not currently
> accepted. They may be accepted after the CLA acceptance workflow is configured. Issues,
> reproducible bug reports, review, and design discussion remain welcome.

## Before contributing

Read, in order:

1. [`docs/index.md`](docs/index.md)
2. [`docs/vision/project-thesis.md`](docs/vision/project-thesis.md)
3. [`docs/vision/uniqueness.md`](docs/vision/uniqueness.md)
4. [`docs/architecture/invariants.md`](docs/architecture/invariants.md)
5. [`docs/ontology/domain-coverage-matrix.md`](docs/ontology/domain-coverage-matrix.md)
6. relevant subsystem documentation
7. relevant ADRs in `docs/adr/`
8. relevant RFCs in `docs/rfc/`

Multi-stage or architectural work requires an ExecPlan following [`PLANS.md`](PLANS.md). Discuss a
major architectural change in an issue before implementation.

## Development workflow

1. Start from current `main` and keep unrelated work out of the change.
2. Use codebase-memory graph tools for code discovery and dependency tracing where available.
3. Capture a failing test or faithful failing scenario before fixing behaviour. Pure documentation
   changes are reviewed for factual accuracy, command execution, and link validity instead.
4. Implement the smallest change that satisfies the accepted scope.
5. Update tests, TODOs, subsystem documentation, ADRs, RFCs, and the domain coverage matrix when the
   change affects them.
6. Run the applicable validation commands below.
7. Keep commits focused and explain causal, determinism, persistence, observer, and performance
   effects in the pull request.

Do not perform opportunistic simulation or architecture work in an unrelated contribution.

## Architectural requirements

- Do not introduce semantic domain enums for convenient labels.
- Do not use English or another human language as authoritative simulation meaning.
- Do not expose Ground Truth or authoritative entity identities directly to agents.
- Preserve physical access, generic perception, subjective scene, concept, belief, and language
  interpretation boundaries.
- Keep objective body state separate from subjective body schema.
- Keep persistent autobiographical memory separate from active working context.
- Treat geography and biology as causal state.
- Do not let LLMs, the Explanation Engine, observer analytics, or UI mutate authoritative state.
- Do not use fixture/demo constructors in production bootstrap or runtime sessions.
- Use state digests only for identity/equality/divergence, never as physical or semantic distances.
- Benchmark every performance or scale claim with a representative reproducible workload.

## Determinism and provenance

Authoritative mutation must follow the scheduler's proposal/reduce/commit boundary. Randomness must
come from explicit deterministic streams and must not depend on thread scheduling, system time,
locale, pointer identity, or hash-map iteration.

Significant state changes require causal traces. Tests for authoritative behaviour should cover, as
applicable:

- same-input replay equivalence;
- input-order independence;
- observer-locale independence;
- negative controls and counterfactuals;
- save/resume equivalence;
- causal ancestry and effect provenance;
- resolution promotion/demotion conservation.

Do not replace domain metrics with digest-byte arithmetic or explain missing evidence narratively.

## Validation

The repository pins Rust 1.85.0 and uses Node.js 20.x with pnpm 9.15.9 in CI.

Rust checks:

```bash
cargo metadata --format-version 1 --no-deps
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --workspace --no-default-features
cargo run -p xtask -- ci
```

Frontend checks:

```bash
pnpm install --frozen-lockfile
pnpm lint
pnpm typecheck
pnpm build
```

Audit-tool tests and dependency advisory check:

```bash
node --test tools/audit/test-*.mjs
pnpm audit --audit-level high
git diff --check
```

Run additional targeted tests and benchmarks for the subsystem changed. Do not report unavailable
tools or skipped commands as passing.

## Documentation expectations

- Describe implemented behaviour and known limits; do not promote an experimental contract to a
  mature capability without the required evidence.
- Link authoritative architecture, ADR, RFC, roadmap, or audit sources instead of copying them.
- Update the roadmap or maturity material only when the accepted evidence changes it.
- Keep commands executable from the repository root unless the text says otherwise.
- Preserve terminology: Ground Truth, subjective scene, Explanation Engine, observer, and mana have
  distinct architectural meanings.

## Pull requests and the CLA

Opening a pull request does not accept the CLA. Before any external contribution can be merged, the
contributor must separately accept the existing [Contributor License Agreement](CLA.md) through a
configured electronic-signature or CLA service. The acceptance record must identify the verified
contributor, CLA version, timestamp, and associated pull request or commit.

The CLA:

- does not transfer contributor copyright;
- grants the maintainer additional rights, including other commercial or proprietary outbound
  terms;
- requires accepted public source contributions to remain available under AGPL-3.0-only and
  accepted documentation or other non-code contributions under CC BY-SA 4.0.

The CLA supplements those public licenses; it does not replace them. The CLA itself calls for legal
review before external contributions are accepted.

## Reporting security issues

Do not put credentials, private data, or vulnerability details in a public issue. Follow
[`SECURITY.md`](SECURITY.md).
