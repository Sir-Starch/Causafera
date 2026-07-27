# Contributing to Causafera

Causafera is an author-led free and open-source causal world-simulation engine
(**Experimental pre-alpha**) with strict deterministic and provenance requirements.

External contributions are welcome when they support the project's canonical vision. Direction,
scope, and acceptance remain with the maintainer, and contributing does not automatically create
governance rights — read [GOVERNANCE.md](GOVERNANCE.md) before investing significant effort.
Technically valid work may still be declined because it does not fit the project's vision;
discussing a substantial change in an issue first is the cheapest way to find that out.

> **CLA acceptance is live.** External contributions may be merged once you have accepted the CLA,
> the required checks pass, and the maintainer approves the change. `license/cla` is a required
> status check on `main`, so an unsigned contribution cannot be merged. Opening a pull request does
> not by itself accept the CLA.
>
> How the workflow is wired is recorded in
> [`docs/legal/cla-service-setup.md`](docs/legal/cla-service-setup.md).

## Contribution flow

Once the CLA service is configured, the intended flow is:

1. **Pick the work.** Choose an open TODO from
   [`docs/development/todo-backlog.md`](docs/development/todo-backlog.md), or discuss a substantial
   change in an issue before implementing it.
2. **Read the required documentation.** The architecture and subsystem documents listed under
   [Before contributing](#before-contributing) below.
3. **Implement a focused change.** One bounded scope, no unrelated opportunistic work.
4. **Review every change.** Read the complete diff yourself, whether you wrote it by hand or an AI
   agent generated it. See [AI-assisted contributions](#ai-assisted-contributions).
5. **Run the required validation.** The commands under [Validation](#validation), and any targeted
   tests and benchmarks for the subsystem you changed.
6. **Open a pull request.** Fill in the template honestly, including checks that failed or were not
   run.
7. **Accept the current CLA** through the configured CLA service, if you have not already accepted
   that version.
8. **Merge** happens only after CLA acceptance and the repository checks pass, and only if the
   maintainer accepts the change.

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

## AI-assisted contributions

AI coding agents are explicitly allowed. You may hand an open TODO to an agent and submit the
result. The project itself is developed this way, and there is no penalty, disclosure stigma, or
separate review track for AI-assisted work.

What does not change is who is accountable. The **human contributor** submitting the pull request
remains responsible for:

- **understanding the change** — you can explain what it does and why, without the agent present;
- **reviewing the complete diff** — every hunk, including files you did not expect to be touched;
- **running the validation and reporting it honestly** — including checks that failed, were skipped,
  or could not run in your environment;
- **the right to submit it** — the contribution must be legally yours to license under the
  [CLA](CLA.md), including any third-party code an agent may have reproduced;
- **correcting hallucinated, speculative, or unrelated changes** — invented APIs, fabricated
  benchmark numbers, citations to documents that do not exist, plausible-looking test assertions
  that verify nothing, and drive-by edits outside the stated scope;
- **compliance with this document** — architecture, determinism, provenance, evidence, and
  documentation requirements apply identically to generated code.

"Give any TODO to an agent" is not permission to submit unreviewed generated code. An unreviewed
diff is not a contribution; it is a request that someone else do the review. Pull requests that show
signs of unreviewed generation — unrelated file churn, unverifiable claims, tests that assert
nothing, documentation describing behaviour that does not exist — will be closed rather than
iterated on.

Agents are also subject to the repository's own agent rules in [`AGENTS.md`](AGENTS.md).

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
node tools/audit/validate-capability-audit.mjs links --paths README.md,CONTRIBUTING.md,GOVERNANCE.md,CLA.md,SECURITY.md,SUPPORT.md,CODE_OF_CONDUCT.md
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
can be merged, you must separately accept the [Contributor License Agreement](CLA.md) through the
configured CLA service. The acceptance record identifies your authenticated GitHub identity, the
repository, the exact CLA revision you accepted, and the timestamp. A materially changed CLA is
published as a new revision and requires a new acceptance; earlier acceptances are not carried over.

Only material you intentionally submit for inclusion is a contribution — a pull request, a commit or
patch, or another channel explicitly designated for that purpose. Filing an issue, reporting a bug,
proposing a feature, or taking part in a design discussion does **not** place that material under
the CLA.

Under the CLA you retain your copyright — accepting a contribution does not transfer it. You grant
the maintainer additional rights, including the ability to offer other commercial or proprietary
outbound terms. That possibility is additional to the public licenses and cannot revoke one already
granted: every public release containing your accepted contribution licenses it under AGPL-3.0-only
for functional software material, or CC BY-SA 4.0 for prose and non-functional explanatory
documentation. This is a licensing commitment rather than a hosting one — the maintainer may modify,
replace, or remove a contribution in later versions, which does not affect the license granted to
releases that already contained it.

The CLA supplements those public licenses; it does not replace them, and it grants no governance
rights (see [GOVERNANCE.md](GOVERNANCE.md)).

## Reporting security issues

Do not put credentials, private data, or vulnerability details in a public issue. Follow
[`SECURITY.md`](SECURITY.md).
