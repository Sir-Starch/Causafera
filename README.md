# Ontopolis

A high-performance experimental simulation of a geographically coherent fantasy / isekai world with extreme causal detail.

## What This Is

Ontopolis simulates the co-evolution of physical reality, geography, biology, incomplete observation, subjective causal models, language, concepts, practices, institutions, information-sensitive magic, and historical path dependence.

The city gradually becomes a physical product of its own history.

## What This Is Not

- Not a generic civilization simulator
- Not a Dwarf Fortress clone
- Not an LLM agent town
- Not a procedural story generator
- Not a collection of semantic enums disguised as emergence

## Project Status

**Phase 0: Project Foundation**

The current task is to establish the complete architectural, conceptual, research, documentation, performance, world-model, language, explanation, observer, and AI-agent foundation required for future development.

## Documentation

See `docs/index.md` for the full documentation guide.

## Architecture

- Rust workspace with 20+ specialized crates
- Deterministic simulation kernel
- Headless authoritative engine
- Observer layer with Protocol Buffers
- Desktop UI via Tauri + React + WebGPU

## License

### Code

All source code in this repository—including tests, build scripts, manifests, and project configuration—is licensed under the **GNU Affero General Public License v3.0 only** (AGPL-3.0-only), unless a file is clearly marked otherwise.

See [`LICENSE`](LICENSE) for the full text.

### Documentation

All documentation, markdown files, diagrams, and other non-code written content are licensed under the **Creative Commons Attribution-ShareAlike 4.0 International** (CC BY-SA 4.0), unless a file is clearly marked otherwise. CC BY-SA 4.0 does not apply to software merely because software appears inside or alongside documentation.

See [`LICENSE-CC-BY-SA-4.0`](LICENSE-CC-BY-SA-4.0) for the full text.

### Contributing

Ontopolis is currently a personal hobby and research project. External code contributions are not accepted until the CLA acceptance workflow is configured. Issues, discussion, and feedback remain welcome.

Contributions require separate, recorded acceptance of the [Contributor License Agreement](CLA.md); opening a pull request alone is not acceptance. The CLA supports commercial/proprietary outbound licensing while preserving the public AGPL-3.0-only or CC BY-SA 4.0 availability applicable to each accepted contribution.

### Why AGPL?

Ontopolis is an experimental research project. For recipients of the public AGPL edition, the AGPL preserves source availability for distributed derivatives and modified network services. The Project Maintainer may separately offer commercial or proprietary licenses under the CLA; those licenses do not withdraw the public AGPL edition.
