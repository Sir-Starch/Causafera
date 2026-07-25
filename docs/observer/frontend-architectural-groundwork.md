# Frontend Architectural Groundwork

**Status:** Binding product requirements, visual intent, and architectural constraints
for the observer frontend redesign.

**Audience:** Claude Opus (authoritative frontend agent) and any future frontend contributor.

**Companion documents:**

- [`docs/observer/frontend-redesign-handoff.md`](frontend-redesign-handoff.md) —
  repository orientation, symbols, excerpts, data flows
- [`docs/observer/capability-maturity-map.md`](capability-maturity-map.md) —
  capability matrix and candidate early scope

---

## 1. What Causafera Is

Causafera is an experimental causal world-simulation engine modeling a persistent fantasy world
with rare isekai arrivals. It explores emergent social, biological, linguistic, and magical
outcomes from physical and cognitive lower-level causes. The simulation is deterministic,
reproducible, and headless. The observer is a read-only derived view.

The observer should evolve into a distinctive, modern, highly capable scientific instrument for
investigating a persistent causal fantasy world.

---

## 2. What the Observer Is Not

The observer is **not a game interface**. It must not feel like:

- a conventional strategy-game interface
- a Paradox-style map-mode game
- a fantasy RPG interface
- a cinematic command center
- a fictional spaceship HUD
- a generic business dashboard
- a simplified consumer application
- an ornamental medieval document viewer

It is a serious desktop analytical application built for inspecting a simulation.

---

## 3. Product Identity

The observer's identity emerges from the integration of:

- spatial state
- entities and agents
- simulation time and historical change
- physical and informational processes
- causal provenance and resolution
- objective world state vs subjective knowledge and belief
- conflicting interpretations
- structured Explanation output
- evidence, uncertainty, and insufficiency
- reports, tables, metrics, diagnostics
- visual projections

Textual and structured data are primary parts of the experience. Visualization extends
analytical understanding rather than replacing it with spectacle.

---

## 4. Product Priorities (Ordered)

1. Usability
2. Analytical efficiency
3. Information density
4. Configurability
5. Scientific clarity and credibility
6. Modern desktop interaction
7. Strong and coherent visual identity

High information density is not a problem. The intended user should be able to inspect,
compare, filter, query, configure, cross-reference, navigate provenance, examine uncertainty,
study detailed reports, explore large quantities of structured data, and use several related
analytical surfaces.

Complexity should be structured, navigable, configurable, visually prioritized, internally
consistent, and efficient for repeated use.

The desired "wow" effect comes from the feeling of observing a vast, causally legible world
through an unusually capable scientific instrument. It should appeal to people who enjoy dense
information, many meaningful values on screen, advanced analytical controls, interconnected
visualizations, deep inspection, configurable workspaces, and technically credible
representations.

---

## 5. Desktop Application Expectations

The observer is a resizable desktop application (Tauri 2). It may run:

- in a regular desktop window
- maximized
- on conventional 16:9 or ultrawide displays
- in a narrow side-by-side window
- under different DPI and OS scaling settings

Responsive behavior should reorganize and reprioritize rather than merely shrink a fixed
composition. The application may use multiple views, workspaces, tabs, secondary navigation,
menus, inspectors, advanced tools, and configurable arrangements. It does not need to force
every capability onto one page.

There should eventually be a strong primary or introductory experience that communicates
the simulation's identity and depth, while detailed analysis lives in specialized areas.

**Classification:** User product intent. Claude Opus determines the concrete solution.

---

## 6. Visualization Expectations

Visualization is important but the observer should not become merely a map viewer.

**Favor:**
- 2D visualization, restrained 2.5D
- diagrams, geometric projections, vector representations
- contours, fields, symbolic markers
- maps, plots, tables, analytical graphics
- restrained local wireframes or sections where genuinely useful

**3D only where it materially improves understanding.**

**Do not orient the frontend around:**
- a rendered game world
- a conventional 3D camera
- textured game terrain
- decorative animated characters
- game-like object rendering

The project should create an impressive world-observation experience through data, geometry,
fields, vectors, symbols, reports, causality, and historical change — not conventional
game graphics.

**Classification:** User product intent.

---

## 7. Binding Creative Direction

> **modern scientific instrumentation + midnight cartography + restrained medieval fantasy
> + the spirit of exploration, discovery, surveying, and terra incognita**

The observer should evoke:

- scholars studying an unfamiliar natural order
- cartographers recording an incompletely known world
- surveyors measuring terrain and phenomena
- natural philosophers classifying physical, biological, cognitive, social, and metaphysical processes
- explorers gradually revealing a world whose mechanisms are real, coherent, and partially understood
- an Age of Discovery expedition interpreted through a modern scientific application

The user should feel that they are revealing, measuring, comparing, tracing, classifying,
documenting, and gradually understanding a persistent world.

This exploration character applies to the **entire application**, not only a geographic map.

**Classification:** User product intent. Claude Opus retains authority over exact expression.

---

## 8. Visual Motifs — Creative References

### 8.1 Dark terra incognita foundation

The application background should evoke a dark or near-black terra incognita map surface:

- dark chart material, black or midnight paper
- subtle fibres or grain
- barely visible geographic or survey structure
- incomplete chart markings, restrained contour traces
- faint coordinates
- layered depth between mapped and unmapped information

This treatment may influence the wider application shell, empty space, transitions, panels,
and unavailable or unexplored states — not merely the map viewport.

Claude Opus must determine how to achieve this without harming readability, contrast,
accessibility, performance, modernity, or visual clarity. This is not a requirement for
literal paper textures or heavy skeuomorphism.

### 8.2 Cartographic influence across the whole UI

The entire interface should carry a restrained cartographic accent. Reference motifs:

- chart boundaries, atlas divisions, survey lines
- coordinate guides, route traces, measurement marks
- map legends, marginal annotations
- contour-like separation, layered sheets
- technical cartographic symbols
- compass or orientation references
- discovery or mapped-state transitions

Claude Opus decides which motifs are useful and how strongly they appear.

### 8.3 Controls and interaction character

Creative reference ideas:

- dotted or dashed outlines
- borders that draw, trace, connect, or complete during hover
- route-like or survey-like motion
- selection states resembling coordinate locks, chart marks, or measured targets
- progress or activity represented as mapped paths
- interaction feedback that feels precise, exploratory, and cartographic

Claude Opus must decide where these motifs improve the design and where conventional
modern controls are clearer. Every interactive element must remain immediately
understandable.

### 8.4 Scientific and medieval balance

The interface is fundamentally modern and scientific. Medieval or early-modern fantasy
influence appears as a restrained accent:

- cartographic composition
- natural-philosophy diagrams
- atlas conventions
- engraved or manuscript-inspired hierarchy
- classification marks, symbolic geometry
- cosmological diagrams
- scientific notation applied to mana and metaphysical phenomena

**Must not become:**
- a parchment menu
- an RPG inventory
- ornamental medieval frames
- fake runes
- a manuscript replica
- an antique interface that sacrifices efficiency

### 8.5 Fantasy relevance

Fantasy character arises from the nature of the simulated world and the phenomena being
studied. It may be expressed through scientific observation of:

- mana fields and resonant patterns
- metaphysical phenomena
- unfamiliar biology
- emergent practices
- competing cosmologies
- unusual causal structures
- incomplete knowledge
- historically situated classifications

Avoid arbitrary fantasy decoration unrelated to actual simulation meaning.

### 8.6 Modernity and density

Despite the exploratory atmosphere, the application must remain:

- modern, sharp, technically credible
- efficient, suitable for dense information
- responsive, precise, readable over long sessions

Do not let atmosphere reduce useful information density. Do not make panels enormous for
visual drama. Do not treat a busy interface as a failure when its density is well structured.

---

## 9. Visual Exclusions

The desired direction explicitly rejects:

- neon cyberpunk
- generic science-fiction HUD styling
- glowing cockpit interfaces
- excessive bloom or glassmorphism
- ornamental fantasy borders
- literal parchment interfaces
- fake runes without semantic meaning
- heavy skeuomorphism
- game-like resource bars and map modes
- casual mobile-style simplification
- visual spectacle that obscures data
- excessive 3D
- a generic dark admin dashboard with a gold accent applied afterward

---

## 10. Architecturally Mandatory Constraints

These are binding regardless of design decisions:

| Constraint | Source |
|-----------|--------|
| UI is a read-only observer — never modifies simulation state | INV-013, INV-021 |
| Rendering representation is not simulation state | INV-022 |
| No privileged UI language — locale cannot change state hash | INV-006, INV-007 |
| Explanations expose confidence and provenance | INV-026 |
| Digests are identity, not distance metrics | INV-038 |
| No demo/fixture data in production | INV-039 |
| LLMs forbidden until terminal gate | INV-011 + rebaseline |
| Observer classifications cannot feed back into simulation | INV-013 |
| Chart-qualified coordinates — no seamless global Cartesian map | INV-036 |
| Geometry ≠ containment ≠ resolution | INV-037 |
| Observer overhead must be bounded | `docs/architecture/performance.md` |
| Protocol is versioned; breaking changes need new version | `docs/architecture/protocol.md` |
| Scoped subscriptions — closed panel = no updates | `docs/observer/backpressure.md` |
| No unbounded observer queues | `docs/observer/backpressure.md` |

---

## 11. Technically Constrained (Current Limitations)

These are current realities, not permanent constraints:

| Limitation | Current state | May evolve when |
|-----------|--------------|----------------|
| Only 3 query kinds (RuntimeSummary, ExplanationIr, WorldChunks) | Protocol v1 | New queries added |
| World projection bounded to active chunks (~3 in demo config) | Runtime config | Config change or larger world |
| Material surface deltas bounded to 64 | Observer API constant | Constant could be revised |
| Single runtime stream (capacity-1, latest-state-wins) | Session.rs | More streams subscribed |
| Explanation limited to material-surface loop experiment | Runtime method | New explanation query types |
| No streaming subscriptions (polling only) | Current Tauri bridge | Event channel implementation |
| No historical state access | No persistence queries | Persistence maturity |
| World data is request/response, not streaming | Current implementation | Stream subscription |
| Two locales only (ru-RU, en-US) | i18n dictionaries | New translations added |

---

## 12. Causal and Epistemic Identity

The architecture distinguishes:

| Concept | Status | Relevant docs |
|---------|--------|---------------|
| **Authoritative state** — Ground Truth maintained by simulation | Fully implemented | `docs/architecture/invariants.md` |
| **Derived observer projections** — read-only summaries | Implemented for summary, chunks, explanation | `docs/architecture/observer.md` |
| **Committed causal events** — provenance graph | Implemented in `CausalTraceStore` | `docs/architecture/provenance.md` |
| **Trace ancestry** — parent-child causal chains | Store supports traversal, no observer query | Same |
| **Evidence** — typed claims with evidence state | Implemented in Explanation IR | `docs/explanation/explanation-ir.md` |
| **Uncertainty** — explicit `Unknown` and `Unsupported` states | Implemented in `ClaimEvidenceState` | `docs/explanation/confidence.md` |
| **Causal insufficiency** — honest "insufficient evidence" | Implemented — claims remain Unknown when evidence missing | Same |
| **Objective vs subjective state** — structural separation | Architecturally established, not projected to observer | INV-001, INV-002, INV-029 |
| **Agent knowledge** — subjective scene, beliefs, concepts | Cognition crate at M1-M2 | `docs/architecture/cognition-rebaseline.md` |
| **Conflicting beliefs** — different agents form different explanations | Architecturally specified | INV-041 |
| **Informational lineage** — testimony chains | Documented, not implemented | `docs/architecture/provenance.md` |
| **Deterministic Explanation** — template-based rendering | Implemented in Rust | `docs/explanation/deterministic-rendering.md` |
| **Optional LLM wording** — after terminal gate only | Forbidden until gate passes | `docs/explanation/optional-llm-surface.md` |
| **Mana as information-sensitive substrate** — physical patterns, not beliefs | Implemented at M3-M4 | INV-003, INV-004 |

**Semantics that must not be lost in UI presentation:**
- Objective state ≠ agent knowledge (even when only objective is currently displayed)
- Explanation claims have typed evidence states — Unknown is meaningful, not a bug
- Digest identity ≠ physical similarity
- Observer labels are not simulation truth
- Absence of evidence ≠ negative evidence

**Representations that would be misleading:**
- Showing chunk coordinates as a seamless global map
- Presenting digest distance as physical similarity
- Displaying Explanation claims without evidence state
- Implying agent knowledge when showing objective state
- Substituting placeholder data when real data is unavailable

---

## 13. Capability-Aware Frontend Requirements

The early frontend must coexist honestly with uneven domain maturity.

| State | Meaning |
|-------|---------|
| **Fully supported** | Real data available through current protocol |
| **Partially supported** | Narrow vertical slice available |
| **Bounded vertical slice** | Working but limited to specific demo config |
| **Prototype/preview** | Exploratory, explicitly labeled |
| **Planned** | Documented architecture, no implementation |
| **Unavailable — domain immature** | Simulation domain not deep enough |
| **Unavailable — observer lacks projection** | Data exists in runtime but no observer query |
| **Insufficient evidence** | Observer tried but evidence is genuinely insufficient |

Claude Opus must design how these states are visually represented. Prototype or preview data
must remain explicitly separate from real observer output. The frontend must never silently
substitute fictional data when the runtime cannot provide authoritative or derived data.

---

## 14. Opus Authority Over Creative Direction

The visual vision above is binding as product intent, atmosphere, and desired identity.
Its exact implementation is not predetermined. Claude Opus may decide:

- which motifs to use, reinterpret, or omit
- how subtle or prominent the cartographic layer should be
- how to balance map texture with clean analytical surfaces
- how controls should behave
- how responsive layouts should reorganize
- what typography, colors, geometry, and motion best express the intent
- whether a proposed motif should be replaced by a stronger solution
