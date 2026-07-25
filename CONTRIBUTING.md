# Contributing to Causafera

Causafera is an experimental causal world-simulation engine (**Experimental pre-alpha**) with
strict deterministic and provenance requirements.

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
2. Capture a failing test or faithful failing scenario before fixing behaviour. Pure documentation
   changes are reviewed for factual accuracy, command execution, and link validity instead.
3. Implement the smallest change that satisfies the accepted scope.
4. Update tests, TODOs, subsystem documentation, ADRs, RFCs, and the domain coverage matrix when the
   change affects them.
5. Run the applicable validation commands below.
6. Keep commits focused and explain causal, determinism, persistence, observer, and performance
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

Do not substitute cryptographic hashes for domain metrics, and do not use narrative claims to
bypass missing causal evidence.

## Validation

The repository pins Rust 1.97.1 and uses Node.js 20.x or 22.x with pnpm 9.15.9 in CI.

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
node tools/audit/check-entry-points.mjs
node tools/audit/run-source-tests.mjs
node tools/audit/validate-capability-audit.mjs links --paths README.md,CONTRIBUTING.md,SECURITY.md,SUPPORT.md,CODE_OF_CONDUCT.md
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

Submitting a pull request does not automatically accept the CLA. Before an external contribution
can be merged, you must separately accept the [Contributor License Agreement](CLA.md) through a
configured electronic-signature or CLA service. The acceptance record must identify the verified
contributor, CLA version, timestamp, and associated pull request or commit.

Under the CLA you retain your copyright, but you grant the maintainer additional rights including
other commercial or proprietary outbound terms. Accepted functional software material — source,
scripts, schemas, manifests, CI configuration, and machine-readable software configuration —
remains available under AGPL-3.0-only. Accepted prose and non-functional explanatory documentation
remains available under CC BY-SA 4.0.

The CLA supplements those public licenses; it does not replace them. The CLA itself calls for
legal review before external contributions are accepted.

## Reporting security issues

Do not put credentials, private data, or vulnerability details in a public issue. Follow
[`SECURITY.md`](SECURITY.md).
