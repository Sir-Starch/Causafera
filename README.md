# Ontopolis

A high-performance experimental simulation of a geographically coherent fantasy / isekai world with extreme causal detail.

Ontopolis simulates the bottom-up co-evolution of physical reality, geography, biology, incomplete observation, subjective causal models, language, concepts, practices, institutions, information-sensitive magic (mana), and historical path dependence.

The simulated city gradually becomes a physical product of its own history.

---

## 🌟 The Core Thesis

Ontopolis is built around a unique, non-trivial simulation thesis:
1. **Subjective Causal Models:** Simulated societies and agents continuously construct subjective models of causality based on lossy, incomplete sensory inputs.
2. **Behavioral Structures:** Their beliefs alter their behavior. Repeated behaviors produce persistent physical and informational structures (such as geometries, frequencies, synchronization patterns, and stable acoustic signals).
3. **Magic is Physical, Not Semantic:** The local magical substrate (**mana**) is a field that reacts to actual, physical patterns and structures (such as repetition, geometry, frequency, spatial recurrence, and persistent symbols). Mana does not understand semantic concepts (it cannot inspect belief states, and has no concept of "gods," "law," "skills," "levels," or "marriage").
4. **Causal Coherence:** Geography, biology, language, and history participate in modifying the future causality that societies later attempt to understand. Every surprising phenomenon is traceably reconstructable to its bottom-up origins (causal provenance).

---

## 🚫 What Ontopolis Is Not

Ontopolis rejects typical shortcuts found in generic simulation games and agent frameworks:
- **Not a generic civilization simulator:** No high-level abstractions or random event tables (e.g., "spawn war" or "spawn plague" events).
- **Not a Dwarf Fortress clone:** Focused on deep epistemic, cognitive, and linguistic emergence rather than macro fortress management.
- **Not an LLM agent town:** LLMs do not run the agent minds or mutate the authoritative state. LLMs are strictly confined to the explanation/UI presentation layer to improve readability.
- **Not a procedural story generator:** Stories are not pre-authored templates; they emerge purely from deterministic physical, biological, and cognitive systems.
- **Not a collection of semantic enums:** The engine does not start with concepts like `Class`, `Skill`, `Level`, or `Disease`. These must emerge from lower-level physical and biological interactions.

---

## ⚙️ Key Unique Features

### 1. Subjective Perception vs. Ground Truth (INV-001 / INV-027)
Agents do not have direct access to Ground Truth. They observe the world through incomplete, physical sensory acquisition. Authoritative identifiers (e.g., `EntityId`, `PlaceId`, `BodySegmentId`) are invisible to agents. Instead, agents construct a **Subjective Scene** containing object identity hypotheses, self-models, and a body schema that can be incorrect, leading to mistakes, misunderstandings, and learning.

### 2. Physicality of Language (INV-008)
Language is physical. Speech acts produce physical acoustic and temporal patterns. Because mana reacts to physical repetition and frequency, changes in language (such as semantic drift or vowel shifts) directly impact magic. For example, a historical sound shift in a language might destabilize a traditional spell, which the inhabitants might interpret as divine anger or declining discipline.

### 3. Causal Geography and Provenance (INV-009 / INV-014)
Geography is causal, not decorative. Materials maintain strict physical provenance from their geological deposit to their placement in a building. If a building collapses, the structural failure can be traced back to the specific quarry and extraction batch of the stone used.

### 4. Objective Biology vs. Subjective Body Schema (INV-034)
Biological structure and variation are simulated down to body segments, joint limits, and physiological state. The agent's cognition experiences this body through a constructed subjective schema (proprioception, balance, pain) which may differ from objective state (e.g., due to fatigue, injury, or phantom limb phenomena).

---

## 🗺️ Roadmap & Current Status

Ontopolis is developed in strict sequential phases to ensure structural integrity and prevent speculative architecture. 

**Current Status:** **Phase 8 Completed / Entering Phase 9 (Subjective Scene Construction)**.

### Completed Foundations:
- **Phases 1–2 (Core & Ontology):** Deterministic scheduler, fixed-point math, coordinate primitives, physical properties (temperature, materials), and generic feature representations (periodicity, relations, magnitudes).
- **Phases 3–4 (World & Geography):** Dense spatial containment hierarchy, terrain cells, geology/hydrology/climate contracts, terrain generator with full provenance verification.
- **Phase 5 (Biology & Pathogens):** Body segment kinematics, joint limits, pathogen lineage transmission, and host-interaction contracts.
- **Phases 6–8 (Causal Perception):** Ground Truth event provenance tracking, physical access signal apertures, generic feature extraction, and bounded attention primitives.

### Upcoming Milestones:
- **Phase 9 (Subjective Scene Construction):** Implementing the sparse subjective scene model (see `RFC-COG-001`) that acts as an intermediate layer between raw sensory features and concepts.
- **Phase 10 (Cognitive Continuity):** Bounded active context, working memory, and prediction error drivers.
- **Phases 11–12 (Concepts & Beliefs):** Sparse concept formation and subjective causal inference.

---

## 📂 Project Structure

Ontopolis is a Rust workspace containing 23 specialized crates, complemented by a Tauri UI and Protobuf definitions:

- **`crates/ontopolis-core`**: The deterministic phase-aware scheduler, random streams, and runtime configuration.
- **`crates/ontopolis-types`**: Common coordinate systems, physics structures, generic features, and typed IDs.
- **`crates/ontopolis-perception`**: Sensor apertures, physical accessibility filters, and feature extraction.
- **`crates/ontopolis-geography`**: Terrain fields and generation provenance.
- **`crates/ontopolis-biology`**: Structural anatomy, joint constraints, and pathogen biology.
- **`crates/ontopolis-cognition`**: Bounded attention, memory, and cognitive state stubs.
- **`crates/ontopolis-explanation`**: The non-authoritative Explanation Engine that converts raw simulation state to structured Explanation IR.
- **`crates/ontopolis-cli`**: Developer command-line interface.
- **`apps/observer`**: Tauri-based desktop app using React and WebGPU to visualize the simulation.
- **`packages/observer-protocol`**: gRPC/Protobuf contracts separating the authoritative simulation from the observer interface.

For a full guide to the documentation index, see `docs/index.md`.

---

## 🛠️ Getting Started

### Prerequisites
- Stable Rust toolchain (configured via `rust-toolchain.toml`)
- [just](https://github.com/casey/just) command runner (optional, but recommended)

### Build and Test
To run the full suite of deterministic tests and formatting lints:
```bash
# Run tests
just test
# Or: cargo test --workspace --all-features

# Run CI check (compiles workspace, formatting, clippy lints)
just ci
# Or: cargo xtask ci
```

### Diagnostics
You can run the simulation's diagnostic tool to verify the local setup:
```bash
just doctor
# Or: cargo run --bin ontopolis -- doctor
```

---

## 📜 Development Rules & Guidelines

All development (including AI agent code) must adhere to the rules in `AGENTS.md`:
- **Deterministic Execution:** The engine must remain 100% deterministic. RNG streams must be keyed by seed, phase, tick, and system.
- **Strict Boundaries:** Authoritative simulation state must never contain human linguistic meaning (such as English labels). Glosses are added strictly by the Explanation Engine.
- **No Omniscience:** Never pass Ground Truth or global IDs directly to agent cognition. Agents must interact with the world through physical perception.
- **codebase-memory-mcp:** Use the codebase memory graph tools (`search_graph`, `trace_path`) as the primary method of code discovery and navigation.

---

## ⚖️ License

- **Code:** Licensed under the [GNU Affero General Public License v3.0 only (AGPL-3.0-only)](LICENSE).
- **Documentation:** Licensed under the [Creative Commons Attribution-ShareAlike 4.0 International (CC BY-SA 4.0)](LICENSE-CC-BY-SA-4.0).

For details on contributing, CLA requirements, and external PR policies, see [CONTRIBUTING.md](CONTRIBUTING.md).
