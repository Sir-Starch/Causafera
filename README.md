# Causafera

Causafera is an experimental causal world-simulation engine intended to model a persistent
fantasy world with rare arrivals from other worlds (isekai). It studies how physical processes,
bounded subjective agents, language, institutions, history, and an information-sensitive mana
field interact to produce traceable, reconstructable outcomes.

> **Status: Experimental pre-alpha.** This repository is public source, not a product, binary
> distribution, production service, or scientific-model release. The Foundation Era contracts and
> a bounded causal loop are implemented; simulation depth is uneven and many domains remain
> incomplete.

## What Causafera is

Causafera explores a specific question: can a persistent world produce surprising social,
linguistic, biological, geographic, and magic-like outcomes from lower-level causes — while
retaining enough provenance to explain what happened?

The intended setting is a coherent fantasy world rather than a generic real-world model. Magic,
cultures, languages, institutions, technologies, and history should arise through the same causal
framework rather than preset narrative or semantic rules.

Rare isekai arrivals — individuals displaced from other worlds — are part of the world model.
They are physical and historical events within the simulation, not protagonists exempt from its
rules. Memories and outside knowledge do not automatically grant capabilities, technologies,
authority, skills, or immunity from local causality.

World state persists independently of any observer or inhabitant. Agents do not read that
authoritative state directly. They receive bounded physical signals and construct subjective
scenes, memories, concepts, beliefs, and causal hypotheses that may be incomplete or wrong.
Their interpretations influence behaviour; repeated behaviour creates physical and informational
patterns; those patterns feed back into later world state.

The central causal loop:

```text
persistent world state
    -> bounded physical access and perception
    -> subjective interpretation and causal hypotheses
    -> behaviour and repeated practices
    -> durable physical or informational patterns
    -> cross-domain effects, including mana response
    -> changed observations and future beliefs
```

Mana is an information-sensitive physical substrate. It responds to measurable recurrence, timing,
synchronization, geometry, frequency, and persistent structure — not to prayer, law, professions,
words, beliefs, classes, skills, or other semantic labels.

## Why causal depth matters

When a simulation routes effects through semantic shortcuts — "the blacksmith has Smithing 5, so
the sword gets +2 quality" — outcomes are predetermined by the label vocabulary. Causafera
replaces that pipeline with layered physical, cognitive, and informational causation where each
domain carries its own state and communicates through causal carriers rather than labels.

This makes outcomes reconstructable. When something unexpected happens, the provenance chain
traces back through perception, belief formation, behavioural decisions, material processes, and
geographic conditions to specific prior causes — including causes the agents themselves
misunderstood.

## Design principles

- **Persistent, observer-independent state.** The authoritative world exists and changes regardless
  of what agents or users know about it.
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
- **Non-authoritative observation.** The observer protocol, desktop UI, classifications, locale,
  and visual presentation are read-only derived tooling — they cannot become simulation truth.
- **Evidence before scale or emergence claims.** Digests prove equality or divergence, not physical
  distance. Performance and emergence claims require representative reproducible evidence.

The non-negotiable rules are maintained in the
[architecture invariants](docs/architecture/invariants.md).

## What exists today

The completed Foundation Era (Phases 0–26) provides minimum validated contracts and selected
executable paths. It does not represent mature simulation depth across every domain.

**Working infrastructure:**

- a Rust 2024 workspace with deterministic scheduler phases and explicit random-stream rules;
- append-only causal events and provenance, canonical state digests, replay checks, and
  deterministic snapshot save/resume;
- a Tauri 2 and React desktop observer consuming versioned Protocol Buffer data rather than
  direct runtime storage.

**Bounded domain contracts** — minimum validated type structures, invariants, and boundaries for
physical space, geography, biology, perception, subjective scenes, cognition, language, practices,
epistemics, social records, economy, city infrastructure, historical bootstrap, isekai transfer,
and metaphysical experiments. These contracts establish architectural boundaries; most do not yet
represent deep simulation.

**Executable paths** — a fixed-point mana field responding to non-semantic recurrence,
synchronization, timing, and spatial pattern structure; causal-resolution and long-run experiment
infrastructure; Explanation IR; observer protocol; and bounded observer overhead measurement.

Current maturity levels and gaps are listed in the
[domain coverage matrix](docs/ontology/domain-coverage-matrix.md).

## What remains incomplete

Major gaps include:

- durable geology, hydrology, climate, ecology, materials, energy, and multiscale geographic
  processes;
- deeply integrated morphology, physiology, development, heredity, reproduction, disease,
  demography, and population conservation;
- long-lived cognition, grounded learning, conversation, language diffusion, institutions,
  production, maintenance, governance, and historical synthesis;
- validated cross-domain mana effects over representative physical carriers;
- production bootstrap paths free of fixture/demo construction;
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

The workspace is split into domain crates. `causafera-types`, `causafera-core`, and the domain
crates define validated primitives and operations; `causafera-runtime` composes authoritative
execution; `causafera-persistence`, `causafera-lab`, `causafera-explanation`, and the observer
crates provide save/resume, experiments, interpretation, and read-only inspection. See the
[documentation index](docs/index.md) and
[Detailed Development rebaseline](docs/architecture/detailed-development-rebaseline.md).

## Prerequisites

- Git;
- Rust 1.97.1 with `rustfmt` and Clippy, from `rust-toolchain.toml`;
- Node.js 20.x or 22.x (Node.js 21.x is outside the locked Vite toolchain's supported engine
  range);
- pnpm 9.15.9.

Building the desktop observer on Linux additionally requires GTK 3, WebKitGTK 4.1, Ayatana
AppIndicator 3, and librsvg development packages. Other platforms require the normal
[Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/).

## Quick start

```bash
cargo metadata --format-version 1 --no-deps
cargo run --bin causafera -- doctor
cargo test --workspace --all-features
pnpm install --frozen-lockfile
pnpm build
```

The `doctor` command verifies core runtime identities and determinism prerequisites. The test
suite is substantial; the repository is experimental and does not install a playable game.

## Desktop observer

After installing dependencies and the platform prerequisites:

```bash
pnpm --dir apps/observer desktop
```

The process is long-running; stop it with `Ctrl+C`. Browser-only Vite mode intentionally cannot
replace the Tauri transport with demonstration data. See the
[observer application guide](docs/ui/observer-application.md) for data-flow and platform details.

## Validation

Rust formatting, linting, and tests:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --workspace --no-default-features
cargo run -p xtask -- ci
```

Frontend:

```bash
pnpm install --frozen-lockfile
pnpm lint
pnpm typecheck
pnpm build
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full validation suite including audit-tool tests
and dependency advisory checks. The GitHub workflow runs equivalent Rust and frontend gates.

No advisory-free, supported binary, or production-security claim is made. Known dependency
advisories and persistence threat-model limitations are documented in [SECURITY.md](SECURITY.md).

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

Causafera is in the open-ended Detailed Development Program. Work is sequenced by accepted
bounded ExecPlans rather than a promised final phase number:

1. deepen authoritative simulation and real cross-domain coupling;
2. keep Explanation and analytics causally inspectable as capabilities mature;
3. add bounded observer read models required for validation;
4. batch coherent UI milestones after read models stabilize;
5. consider optional LLM surface wording only after the terminal maturity gate.

See the [roadmap](docs/roadmap/roadmap.md) for the authoritative status and the
[documentation index](docs/index.md) for the full documentation tree.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development workflow, architectural requirements,
validation commands, and CLA details.

External code contributions are not currently accepted until the CLA acceptance workflow is
configured. Issues and evidence-backed design discussion remain welcome.

- [Security policy](SECURITY.md) — vulnerability reporting and known limitations
- [Support guidance](SUPPORT.md) — issue filing and scope
- [Contributor License Agreement](CLA.md)

## Licenses

- Functional software material — Rust, JavaScript, TypeScript, scripts, schemas, manifests, CI
  configuration, and machine-readable software configuration — is licensed under
  [GNU AGPL v3.0 only](LICENSE) (`AGPL-3.0-only`).
- Prose and non-functional explanatory documentation is licensed under
  [Creative Commons Attribution-ShareAlike 4.0 International](LICENSE-CC-BY-SA-4.0)
  (`CC BY-SA 4.0`), unless a file states otherwise.
- Third-party dependencies retain their own licenses.
- Contributions are governed by the existing [CLA](CLA.md) plus the applicable public outbound
  license. The CLA does not replace those licenses or transfer contributor copyright.
