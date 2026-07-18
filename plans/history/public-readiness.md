# Causafera Public Readiness

> **Historical record.** This completed 2026 public-source-readiness plan records an audit and implementation snapshot, not current repository status. Its environment-specific evidence may be outdated; use [the documentation index](../../docs/index.md), [README.md](../../README.md), and [active plans](../../PLANS.md) for current guidance.

**Status:** Completed for public source visibility; not a product or binary release

**Starting commit:** `6af6e808c3e3cb3f7b5a6107a547a18010fb103b`

**Working branch:** `release/public-readiness`

## Goal

Prepare the repository for a future public visibility change by making its public documentation,
community files, licensing metadata, security posture, and local validation instructions factual
and reproducible. This plan does not publish, rename, push, merge, or rewrite the repository.

## Context

Causafera has completed its Foundation Era but remains an experimental pre-alpha with uneven domain
maturity. The repository is currently private. Source code is intended to be AGPL-3.0-only,
documentation CC BY-SA 4.0, and accepted contributions subject to the existing CLA in addition to
the public outbound licenses.

## Relevant invariants

- INV-001 and INV-002: agents cannot access Ground Truth.
- INV-003 and INV-004: mana reacts to physical or informational structure, not beliefs or meaning.
- INV-011 through INV-013: LLM, Explanation, and observer surfaces are non-authoritative.
- INV-014 and INV-019: significant changes and claims require causal provenance.
- INV-018: performance claims require reproducible benchmarks.
- INV-038 and INV-039: digests are identities, and production state requires causal initialization.

## Ontology domains affected

None. This is repository governance, documentation, build metadata, dependency security, and CI
hardening only.

## Causal carriers affected

None.

## Relevant documents

- `README.md`
- `CONTRIBUTING.md`
- `SECURITY.md`
- `CLA.md`
- `docs/architecture/invariants.md`
- `docs/architecture/detailed-development-rebaseline.md`
- `docs/ontology/domain-coverage-matrix.md`
- `docs/roadmap/roadmap.md`
- `docs/ui/observer-application.md`
- `PLANS.md`

## Current state

- The initial branch point is a clean `main` at the starting commit above.
- The README is a long vision document and does not provide the requested concise public entry point
  or complete frontend validation path.
- Security, conduct, support, citation, issue, and pull-request community files are absent.
- Cargo workspace license metadata is AGPL-3.0-only, but JavaScript package manifests omit license
  metadata.
- CI uses mutable action tags and does not declare explicit permissions.
- `pnpm audit --audit-level high` reports one high and three moderate development-server advisories;
  the patched Vite line begins at 6.4.3.
- gitleaks, cargo-audit, and cargo-deny are not installed and must not be described as passing.

## Proposed architecture

No simulation architecture changes. Public documentation will point to authoritative internal
documents rather than duplicating them. Community policy will be represented by useful Markdown,
YAML templates, and manifest/workflow metadata. CI will retain the existing jobs while pinning
permissions, tool versions, and third-party action commits.

## Primitive vs emergent review

Not applicable to repository-governance changes. Documentation must continue to describe semantic
fantasy categories as possible emergent outcomes rather than authoritative primitives.

## Non-goals

- Changing repository visibility or GitHub settings.
- Renaming the GitHub repository.
- Pushing, merging, rebasing, squashing, amending, or otherwise rewriting history.
- Product, architecture, observer-protocol, persistence, or simulation changes.
- Unrelated dependency upgrades or prose polish beyond clarity and accuracy.

## Implementation stages

1. Capture the starting Git state and requirement-specific failing checks.
2. Audit full history for secrets, private data, stale names, unusual blobs, and generated content.
3. Audit license/CLA consistency, dependency advisories/licenses, and GitHub Actions trust boundaries.
4. Replace the public README and add the justified community files.
5. Add narrow manifest, dependency, ignore, and workflow hardening supported by audit findings.
6. Validate in the working tree, create no more than three focused commits, and validate every
   documented command in a clean detached clone.
7. Run independent exact-SHA review, remove temporary state, and report external GitHub actions
   that remain for a maintainer.

## Verification

- `git diff --check`
- `cargo metadata --format-version 1 --no-deps`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo test --workspace --no-default-features`
- `cargo run -p xtask -- ci`
- `pnpm install --frozen-lockfile`
- `pnpm lint`
- `pnpm typecheck`
- `pnpm build`
- `node tools/audit/check-entry-points.mjs`
- `node tools/audit/run-source-tests.mjs`
- `node tools/audit/validate-capability-audit.mjs links --paths README.md,CONTRIBUTING.md,SECURITY.md,SUPPORT.md,CODE_OF_CONDUCT.md`
- `pnpm audit --audit-level high`
- Redacted full-history secret/privacy scan, license check, Markdown-link check, exact legacy-name
  search, repository-hygiene check, and clean-clone replay of all README commands.

Unavailable checks are recorded with their exact non-zero status and are never reported as passing.

## Benchmark plan

No performance claim or simulation behavior changes. Existing representative benchmarks are not
rerun as evidence for repository documentation.

## Determinism impact

No authoritative execution changes. The documented contributor workflow retains all-features and
no-default-features test coverage and deterministic/provenance review requirements.

## Memory impact

None.

## Observer impact

No protocol or UI behavior changes. Observer startup instructions are included only if verified in
a clean environment; otherwise the README points to the observer documentation without claiming a
successful local desktop launch.

## Explanation impact

None. Public documentation preserves the non-authoritative Explanation boundary and current
maturity limitations.

## Persistence impact

None.

## Cross-domain effects

None.

## Risks

- A historical credential or accidental personal record is a release blocker and requires
  revocation before any history-rewrite decision.
- Public wording may overstate the Foundation Era; the maturity matrix and rebaseline are the
  controlling sources.
- Pinning action commits can silently select the wrong release if tag resolution is not verified
  against the upstream action repository.
- Clean-clone validation may reveal undocumented operating-system dependencies.
- The existing CLA requires a separately configured acceptance workflow and legal review before
  external contributions can be accepted.

## Documentation changes

Replace the root README, expand the root contribution guide, add useful public community files,
and link authoritative architecture, roadmap, maturity, security, contribution, and license
sources.

## TODO changes

No simulation TODO changes. External GitHub configuration and CLA-enablement decisions are listed
in the final report rather than assigned simulation phase numbers.

## Decision log

- 2026-07-18: Classified this pass as repository governance rather than a simulation phase.
- 2026-07-18: Kept the existing CLA text unchanged; explanatory material must not imply that the
  CLA replaces AGPL-3.0-only or CC BY-SA 4.0 public licensing.
- 2026-07-18: Treat unavailable gitleaks, cargo-audit, and cargo-deny as unavailable, not passing.
- 2026-07-18: A high Vite advisory is in scope for narrow security remediation despite the general
  prohibition on unrelated upgrades.
- 2026-07-18: Used the existing maintainer-controlled Git identity address for private security and
  conduct reports; no new contact identity was invented.
- 2026-07-18: Kept 35 old-name occurrences solely as immutable audit-baseline data or explicit
  legacy compatibility constants. No path-name occurrence or current identity remains.
- 2026-07-18: Pinned third-party Actions to upstream-verified immutable commits and retained
  workflow-wide read-only contents permission.
- 2026-07-18: The native observer passed clean-clone startup with a fresh WebKit profile. A first
  run reused persistent development cache from an earlier checkout and showed a blank window; the
  profile was moved aside reversibly, the clean run rendered live data, and the original profile
  was restored. Changing observer cache policy is outside this repository-readiness scope.

## Outcome

The public entry point and useful community files are present, source and documentation licensing
are stated consistently, JavaScript metadata matches the Cargo workspace, known frontend
advisories are remediated, CI uses read-only permissions and immutable action pins, and the
documented public setup commands passed in a clone without inherited dependency or build
directories. This outcome concerns public source visibility only. Known Rust observer advisories
and persistence limitations remain documented pre-alpha constraints before supported binary
distribution, untrusted input, or production use.

No simulation architecture, authoritative state, observer protocol, or UI behaviour changed. The
repository was not pushed, merged, renamed externally, made public, or subjected to history
rewriting. GitHub-hosted security, protection, contribution-acceptance, and visibility settings
remain explicit maintainer actions.

## Progress

- [x] Starting SHA and clean `main` recorded.
- [x] `release/public-readiness` created.
- [x] Required project documentation read.
- [x] Pre-change public-surface, policy, workflow, legacy-name, command, and dependency failures
  captured.
- [x] Full-history secret/privacy and hygiene audits complete.
- [x] License, CLA, legacy, workflow, and dependency audits complete.
- [x] Public documentation and community files implemented.
- [x] Configuration hardening implemented.
- [x] Focused commits created and working-tree validation passed.
- [x] Clean-clone validation passed.
- [x] Final evidence commit prepared for exact-SHA review and temporary-resource cleanup before
  handoff.
