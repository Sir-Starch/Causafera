<div align="center">

<h1 id="causafera">Causafera</h1>

<p><strong>A deterministic causal simulation engine for persistent fantasy worlds.</strong></p>

<p>
  Causafera models how physical processes, bounded subjective agents, language, institutions,<br>
  history, and an information-sensitive mana field interact to produce traceable outcomes.
</p>

<p>
  <a href="docs/index.md"><strong>Documentation</strong></a>
  ·
  <a href="docs/roadmap/roadmap.md"><strong>Roadmap</strong></a>
  ·
  <a href="CONTRIBUTING.md"><strong>Contributing</strong></a>
  ·
  <a href="SECURITY.md"><strong>Security</strong></a>
</p>

<p>
  <a href="docs/roadmap/roadmap.md">
    <img alt="Project status: experimental pre-alpha" src="https://img.shields.io/badge/status-experimental%20pre--alpha-E67E22?style=flat-square">
  </a>
  <a href="rust-toolchain.toml">
    <img alt="Rust 2024" src="https://img.shields.io/badge/Rust-2024-000000?style=flat-square&amp;logo=rust&amp;logoColor=white">
  </a>
  <a href="https://v2.tauri.app/">
    <img alt="Tauri 2" src="https://img.shields.io/badge/Tauri-2-24C8DB?style=flat-square&amp;logo=tauri&amp;logoColor=white">
  </a>
  <a href="LICENSE">
    <img alt="Software license: AGPL-3.0-only" src="https://img.shields.io/badge/software-AGPL--3.0--only-663399?style=flat-square">
  </a>
  <a href="LICENSE-CC-BY-SA-4.0">
    <img alt="Documentation license: CC BY-SA 4.0" src="https://img.shields.io/badge/docs-CC%20BY--SA%204.0-1769AA?style=flat-square&amp;logo=creativecommons&amp;logoColor=white">
  </a>
</p>

<sub>
  <a href="#overview">Overview</a> ·
  <a href="#causal-model">Causal model</a> ·
  <a href="#project-status">Status</a> ·
  <a href="#architecture">Architecture</a> ·
  <a href="#quick-start">Quick start</a> ·
  <a href="#validation">Validation</a> ·
  <a href="#repository-layout">Repository</a> ·
  <a href="#licenses">Licenses</a>
</sub>

</div>

> [!WARNING]
> **Causafera is experimental pre-alpha software.** This repository is source code, not a
> finished product, binary distribution, production service, playable game, or scientific-model
> release. The Foundation Era contracts and a bounded causal loop are implemented, but simulation
> depth remains uneven and many domains are incomplete.

## Overview

Causafera explores a specific question:

> Can a persistent world produce surprising social, linguistic, biological, geographic, and
> magic-like outcomes from lower-level causes while retaining enough provenance to explain what
> happened?

The intended setting is a coherent fantasy world rather than a generic real-world model. Magic,
cultures, languages, institutions, technologies, and history should arise through the same causal
framework instead of preset narrative rules or semantic shortcuts.

Rare **isekai arrivals**, individuals displaced from other worlds, are physical and historical
events within the simulation. They are not protagonists exempt from its rules. Memories and
outside knowledge do not automatically grant capabilities, technologies, authority, skills, or
immunity from local causality.

### At a glance

| Concern | Causafera's approach |
| --- | --- |
| World state | Persistent and observer-independent |
| Agents | Bounded perception, subjective scenes, fallible beliefs |
| Execution | Deterministic scheduling, explicit random streams, reproducible replay |
| Cross-domain effects | Physical and informational causal carriers, not semantic labels |
| Magic | An information-sensitive physical substrate responding to measurable structure |
| Explanation | Typed, evidence-bearing provenance downstream of authoritative state |
| Observation | Read-only derived tooling that cannot become simulation truth |

## Causal model

World state persists independently of any observer or inhabitant. Agents cannot read authoritative
state directly. They receive bounded physical signals and construct subjective scenes, memories,
concepts, beliefs, and causal hypotheses that may be incomplete or wrong.

Those interpretations influence behaviour. Repeated behaviour creates durable physical and
informational patterns, which feed back into later world state:

```text
persistent world state
    -> bounded physical access and perception
    -> subjective interpretation and causal hypotheses
    -> behaviour and repeated practices
    -> durable physical or informational patterns
    -> cross-domain effects, including mana response
    -> changed observations and future beliefs
```

Mana is an information-sensitive physical substrate. It responds to measurable recurrence,
timing, synchronization, geometry, frequency, and persistent structure, not to prayer, law,
professions, words, beliefs, classes, skills, or other semantic labels.

### Why causal depth matters

A shortcut such as "the blacksmith has Smithing 5, so the sword gets +2 quality" predetermines the
outcome through label vocabulary. Causafera replaces that pipeline with layered physical,
cognitive, and informational causation. Each domain owns its state and communicates through causal
carriers rather than labels.

This makes unexpected outcomes reconstructable. A provenance chain can trace an authoritative
change through perception, belief formation, behavioural decisions, material processes, and
geographic conditions to specific prior causes, including causes the agents themselves
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
  and visual presentation are read-only derived tooling and cannot become simulation truth.
- **Evidence before scale or emergence claims.** Digests prove equality or divergence, not physical
  distance. Performance and emergence claims require representative reproducible evidence.

The non-negotiable rules are defined in the
[architecture invariants](docs/architecture/invariants.md).

## Project status

The completed **Foundation Era (Phases 0–26)** provides minimum validated contracts and selected
executable paths. It does not represent mature simulation depth across every domain.

### Implemented foundations

- Rust 2024 workspace with deterministic scheduler phases and explicit random-stream rules.
- Append-only causal events and provenance.
- Canonical state digests, replay checks, and deterministic snapshot save/resume.
- Tauri 2 and React desktop observer consuming versioned Protocol Buffer data instead of direct
  runtime storage (supporting UI localization in English, Russian, Simplified Chinese, German, and Spanish).
- Bounded contracts for physical space, geography, biology, perception, subjective scenes,
  cognition, language, practices, epistemics, social records, economy, city infrastructure,
  historical bootstrap, isekai transfer, and metaphysical experiments.
- Fixed-point mana field responding to non-semantic recurrence, synchronization, timing, and
  spatial pattern structure.
- Causal-resolution and long-run experiment infrastructure, Explanation IR, observer protocol,
  and bounded observer-overhead measurement.

These contracts establish architectural boundaries. Most domains do not yet represent deep
simulation. Current maturity levels are tracked in the
[domain coverage matrix](docs/ontology/domain-coverage-matrix.md).

### Major incomplete areas

- Durable geology, hydrology, climate, ecology, materials, energy, and multiscale geographic
  processes.
- Deeply integrated morphology, physiology, development, heredity, reproduction, disease,
  demography, and population conservation.
- Long-lived cognition, grounded learning, conversation, language diffusion, institutions,
  production, maintenance, governance, and historical synthesis.
- Validated cross-domain mana effects over representative physical carriers.
- Production bootstrap paths free of fixture or demonstration construction.
- Domain-valid recovery metrics, counterfactuals, causal queries, uncertainty reporting, and
  observer inspection coverage.
- Representative long runs, performance envelopes, provenance-growth measurements, and evidence
  for nontrivial emergence.

Optional LLM wording is neither implemented nor scheduled. It remains behind a terminal maturity
gate and would be removable, non-authoritative, and downstream of deterministic Explanation.

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

The workspace is split into domain crates:

- `causafera-types`, `causafera-core`, and the domain crates define validated primitives and
  operations.
- `causafera-runtime` composes authoritative execution.
- `causafera-persistence`, `causafera-lab`, `causafera-explanation`, and the observer crates provide
  save/resume, experiments, interpretation, and read-only inspection.

For the full design, see the [documentation index](docs/index.md),
[architecture invariants](docs/architecture/invariants.md), and
[Detailed Development rebaseline](docs/architecture/detailed-development-rebaseline.md).

## Quick start

### Prerequisites

| Dependency | Required version |
| --- | --- |
| Git | Current supported release |
| Rust | 1.97.1 with `rustfmt` and Clippy, pinned by `rust-toolchain.toml` |
| Node.js | 20.x or 22.x; Node.js 21.x is outside the locked Vite engine range |
| pnpm | 9.15.9 |

Building the desktop observer on Linux additionally requires GTK 3, WebKitGTK 4.1, Ayatana
AppIndicator 3, and librsvg development packages. Other platforms require the standard
[Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/).

### Verify the workspace

From the repository root:

```bash
cargo metadata --format-version 1 --no-deps
cargo run --bin causafera -- doctor
cargo test --workspace --all-features
pnpm install --frozen-lockfile
pnpm build
```

The `doctor` command verifies core runtime identities and determinism prerequisites. The test suite
is substantial; these commands validate the repository but do not install a playable game.

### Run the desktop observer

After installing the platform prerequisites:

```bash
pnpm --dir apps/observer desktop
```

The process is long-running; stop it with `Ctrl+C`. Browser-only Vite mode intentionally cannot
replace the Tauri transport with demonstration data. See the
[observer application guide](docs/ui/observer-application.md) for data flow and platform details.

## Validation

### Rust

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --workspace --no-default-features
cargo run -p xtask -- ci
```

### Frontend

```bash
pnpm install --frozen-lockfile
pnpm lint
pnpm typecheck
pnpm build
```

[CONTRIBUTING.md](CONTRIBUTING.md) contains the complete validation suite, including audit-tool
tests and dependency advisory checks. GitHub Actions runs equivalent Rust and frontend gates.

> [!NOTE]
> The project makes no advisory-free, supported-binary, or production-security claim. Known
> dependency advisories and persistence threat-model limitations are documented in
> [SECURITY.md](SECURITY.md).

## Repository layout

| Path | Purpose |
| --- | --- |
| `crates/` | Authoritative Rust types, domains, runtime, persistence, analytics, CLI, and lab |
| `apps/observer/` | Tauri 2 and React desktop observer |
| `packages/observer-protocol/` | TypeScript Protocol Buffer decoding and observer protocol types |
| `proto/` | Versioned observer Protocol Buffer schemas |
| `tests/` | Cross-crate architecture and determinism tests |
| `tools/xtask/` | Canonical Rust CI orchestration |
| `tools/audit/` | Reproducible maturity-audit schemas, fixtures, and regression tests |
| `docs/` | Vision, invariants, architecture, ontology, subsystems, roadmap, ADRs, and RFCs |
| `plans/` | Accepted, active, draft, and completed ExecPlans |

## Roadmap

Causafera is in the open-ended **Detailed Development Program**. Work is sequenced through accepted,
bounded ExecPlans rather than a promised final phase number:

1. Deepen authoritative simulation and real cross-domain coupling.
2. Keep Explanation and analytics causally inspectable as capabilities mature.
3. Add bounded observer read models required for validation.
4. Batch coherent UI milestones after read models stabilize.
5. Consider optional LLM surface wording only after the terminal maturity gate.

The [roadmap](docs/roadmap/roadmap.md) is the authoritative status source. The
[documentation index](docs/index.md) provides the complete documentation tree.

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) before proposing changes. It defines the development
workflow, architectural requirements, validation commands, and Contributor License Agreement
requirements.

External code contributions are not currently accepted until the CLA acceptance workflow is
configured. Issues and evidence-backed design discussion remain welcome.

| Resource | Purpose |
| --- | --- |
| [Contributing guide](CONTRIBUTING.md) | Development workflow and validation requirements |
| [Security policy](SECURITY.md) | Vulnerability reporting and known limitations |
| [Support guidance](SUPPORT.md) | Issue filing, questions, and project scope |
| [Contributor License Agreement](CLA.md) | Terms governing accepted contributions |

## Licenses

Causafera uses separate licenses for software and explanatory documentation:

- **Software:** Rust, JavaScript, TypeScript, scripts, schemas, manifests, CI configuration, and
  machine-readable software configuration are licensed under
  [GNU AGPL v3.0 only](LICENSE) (`AGPL-3.0-only`).
- **Documentation:** Prose and non-functional explanatory documentation are licensed under
  [Creative Commons Attribution-ShareAlike 4.0 International](LICENSE-CC-BY-SA-4.0)
  (`CC BY-SA 4.0`), unless a file states otherwise.
- Third-party dependencies retain their own licenses.
- Contributions are governed by the existing [CLA](CLA.md) plus the applicable public outbound
  license. The CLA does not replace those licenses or transfer contributor copyright.

---

<div align="center">
  <sub>
    <a href="#causafera">Back to top</a>
  </sub>
</div>
