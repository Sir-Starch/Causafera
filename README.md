# Causafera

Causafera is an experimental causal world-simulation engine for studying how physical processes,
bounded subjective agents, language, institutions, and an information-sensitive mana field can
co-evolve into reconstructable history.

> **Status: Experimental pre-alpha.** Causafera is not a finished game, a production-ready
> simulator, or a completed scientific model. Its Foundation Era contracts and a bounded causal
> loop are implemented, but simulation depth is uneven and many domains remain incomplete.

## What Causafera is

Causafera exists to explore a specific question: can a persistent world produce surprising social,
linguistic, biological, geographic, and magic-like outcomes from lower-level causes while retaining
enough provenance to explain what happened?

The world state persists independently of any observer or inhabitant. Agents do not read that
authoritative state. They receive bounded, physically accessible signals and construct subjective
scenes, memories, concepts, beliefs, and causal hypotheses that may be incomplete or wrong. Their
interpretations influence behaviour; repeated behaviour creates real physical and informational
patterns; those patterns may affect later world state.

The central causal loop is:

```text
persistent world state
    -> bounded physical access and perception
    -> subjective interpretation and causal hypotheses
    -> behaviour and repeated practices
    -> durable physical or informational patterns
    -> cross-domain effects, including mana response
    -> changed observations and future beliefs
```

Mana is an information-sensitive physical simulation substrate. It may respond to measurable
recurrence, timing, synchronization, geometry, frequency, and persistent structure. It does not
understand prayer, law, professions, words, beliefs, classes, skills, or other semantic labels.

## Design principles

- **Persistent, observer-independent state.** The authoritative world continues to exist and change
  regardless of what agents or users know about it.
- **Bounded subjective perception.** Agents never receive Ground Truth identities or complete world
  state. Perception, subjective scene construction, and belief remain structurally separate.
- **Cross-domain causality.** Geography, biology, material processes, cognition, language, society,
  history, and mana exchange physical or informational causal carriers rather than semantic
  shortcuts.
- **Deterministic and reproducible execution.** Explicit random streams, stable proposal ordering,
  canonical state representations, and replay checks support reproducible experiments.
- **Causal provenance and explanation.** Significant authoritative changes retain traceable causes.
  The Explanation Engine renders typed, evidence-bearing interpretations without mutating the
  simulation.
- **Non-authoritative observation.** The observer protocol, desktop UI, classifications, locale, and
  visual presentation are read-only derived tooling and cannot become simulation truth.
- **Evidence before scale or emergence claims.** Digests prove equality or divergence, not physical
  distance. Performance and emergence claims require representative reproducible evidence.

The non-negotiable rules are maintained in the [architecture invariants](docs/architecture/invariants.md).

## Implemented now

The completed Foundation Era provides minimum validated contracts and selected executable paths:

- a Rust 2024 workspace with deterministic scheduler phases and explicit random-stream rules;
- append-only causal events and provenance, canonical state digests, replay checks, and deterministic
  snapshot save/resume;
- bounded contracts for physical space, geography, biology, perception, subjective scenes,
  cognition, language, practices, epistemics, social records, economy, city infrastructure,
  historical bootstrap, isekai transfer, and metaphysical experiments;
- a fixed-point mana field responding to non-semantic recurrence, synchronization, timing, and
  spatial pattern structure, with traced coupling in a limited executable path;
- causal-resolution, long-run experiment, Explanation IR, observer protocol, and bounded observer
  overhead paths;
- a Tauri 2 and React desktop observer that consumes versioned Protocol Buffer data rather than
  direct runtime storage.

Foundation completion does not mean every broad domain is mature. The conservative current levels
and gaps are listed in the [domain coverage matrix](docs/ontology/domain-coverage-matrix.md), and the
[maturity audit plan](plans/detailed-development-maturity-audit.md) remains active.

## Incomplete work

Major gaps include:

- durable geology, hydrology, climate, ecology, materials, energy, and multiscale geographic
  processes;
- deeply integrated morphology, physiology, development, heredity, reproduction, disease,
  demography, and population conservation;
- long-lived cognition, grounded learning, conversation, language diffusion, institutions,
  production, maintenance, governance, and historical synthesis;
- validated cross-domain mana effects over representative physical carriers;
- production bootstrap and detail-promotion paths free of fixture/demo construction throughout;
- domain-valid recovery metrics, counterfactuals, causal queries, uncertainty reporting, and
  observer inspection coverage;
- representative long runs, performance envelopes, provenance-growth measurements, and evidence
  for nontrivial emergence.

Optional LLM wording is not implemented or scheduled. It remains behind a terminal maturity gate
and would be removable, non-authoritative, and downstream of deterministic Explanation.

## Architecture

```text
domain state and physical carriers
    -> deterministic READ / PROPOSE / REDUCE / COMMIT scheduler
    -> authoritative world state plus causal provenance
    -> persistence, replay, experiments, and typed analytics
    -> bounded observer read model and Explanation IR
    -> versioned Protocol Buffer transport
    -> non-authoritative Tauri / React observer
```

The workspace is split into domain crates. `causafera-types`, `causafera-core`, and the domain crates
define validated primitives and operations; `causafera-runtime` composes authoritative execution;
`causafera-persistence`, `causafera-lab`, `causafera-explanation`, and the observer crates provide
save/resume, experiments, interpretation, and read-only inspection. See the
[documentation index](docs/index.md) and [Detailed Development rebaseline](docs/architecture/detailed-development-rebaseline.md).

## Prerequisites

The repository and CI currently pin or verify:

- Git;
- Rust 1.85.0 with `rustfmt` and Clippy, from `rust-toolchain.toml`;
- Node.js 20.x;
- pnpm 9.15.9.

Building the desktop observer on Linux additionally requires GTK 3, WebKitGTK 4.1, Ayatana
AppIndicator 3, and librsvg development packages. Other platforms require the normal
[Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/).

## Quick start

From the repository root:

```bash
cargo metadata --format-version 1 --no-deps
cargo run --bin causafera -- doctor
cargo test --workspace --all-features
pnpm install --frozen-lockfile
pnpm build
```

The doctor command verifies core runtime identities and determinism prerequisites. The test suite
is substantial; the repository is experimental and does not install a playable game.

## Validation

Rust formatting, linting, feature coverage, and the repository CI command:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --workspace --no-default-features
cargo run -p xtask -- ci
```

Frontend installation and validation:

```bash
pnpm install --frozen-lockfile
pnpm lint
pnpm typecheck
pnpm build
```

Audit-tool regression tests and dependency advisory check:

```bash
node --test tools/audit/test-*.mjs
pnpm audit --audit-level high
```

Before committing, also run:

```bash
git diff --check
```

The GitHub workflow runs equivalent Rust and frontend gates. `cargo-audit`, `cargo-deny`, and
gitleaks are useful additional checks but are not bundled with this repository.

## Desktop observer

After installing dependencies and the platform prerequisites, start the native observer with:

```bash
pnpm --dir apps/observer desktop
```

The process is long-running; stop it with `Ctrl+C`. Browser-only Vite mode intentionally cannot
replace the Tauri transport with demonstration data. See the
[observer application guide](docs/ui/observer-application.md) for data-flow and platform details.

## Repository structure

| Path | Purpose |
| --- | --- |
| `crates/` | Authoritative Rust types, domain contracts, runtime, persistence, analytics, CLI, and lab |
| `apps/observer/` | Tauri 2 and React desktop observer |
| `packages/observer-protocol/` | TypeScript Protocol Buffer decoding and observer protocol types |
| `proto/` | Versioned observer Protocol Buffer schemas |
| `tests/` | Cross-crate architecture and determinism tests |
| `tools/xtask/` | Canonical Rust CI orchestration |
| `tools/audit/` | Reproducible maturity-audit schemas, fixtures, and regression tests |
| `docs/` | Vision, invariants, architecture, ontology, subsystem, roadmap, ADR, and RFC documentation |
| `plans/` | Accepted, active, draft, and completed ExecPlans |

## Roadmap

Causafera is in the open-ended Detailed Development Program. Work is sequenced by accepted bounded
ExecPlans rather than a promised final phase number:

1. deepen authoritative simulation and real cross-domain coupling;
2. keep Explanation and analytics causally inspectable as capabilities mature;
3. add bounded observer read models required for validation;
4. batch coherent UI milestones after read models stabilize;
5. consider optional LLM surface wording only after the terminal maturity gate.

See the [roadmap](docs/roadmap/roadmap.md) for the authoritative status.

## Contributing, support, and security

- [Contributing guide](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [Support guidance](SUPPORT.md)
- [Contributor License Agreement](CLA.md)

External code contributions are not currently accepted until the CLA acceptance workflow is
configured. Issues and evidence-backed design discussion remain welcome.

## Licenses

- Software source and software configuration are licensed under
  [GNU AGPL v3.0 only](LICENSE) (`AGPL-3.0-only`).
- Documentation and other non-code project materials are licensed under
  [Creative Commons Attribution-ShareAlike 4.0 International](LICENSE-CC-BY-SA-4.0)
  (`CC BY-SA 4.0`), unless a file states otherwise.
- Third-party dependencies retain their own licenses.
- Contributions are additionally governed by the [CLA](CLA.md). The CLA does not replace the
  public outbound licenses or transfer contributor copyright.
