# Observer Capability Maturity Map

**Status:** Technical readiness assessment for an early major frontend implementation.
Not an authoritative UI plan. Claude Opus may accept, modify, or reject the proposed scope.

**Companion documents:**

- [`docs/observer/frontend-redesign-handoff.md`](frontend-redesign-handoff.md) —
  repository orientation, symbols, data flows
- [`docs/observer/frontend-architectural-groundwork.md`](frontend-architectural-groundwork.md) —
  product intent, visual direction, constraints

**Cross-references:**

- `docs/ontology/domain-coverage-matrix.md` — authoritative domain maturity
- `docs/architecture/detailed-development-rebaseline.md` — maturity model M0-M5

---

## 1. Maturity Model Reference

From `docs/architecture/detailed-development-rebaseline.md`:

| Level | Name | Evidence |
|-------|------|----------|
| M0 | Documented | Intent, boundaries, carriers, risks documented |
| M1 | Contracted | Validated deterministic types and isolated operations |
| M2 | Executable | Production scheduler mutates authoritative state |
| M3 | Coupled | Real cross-domain inputs/outputs exist |
| M4 | Observable | Bounded read models and domain-valid Explanation |
| M5 | Validated | Replay-verified long runs, controls, benchmarks |

---

## 2. Observer-Relevant Capability Matrix

### Stable foundations (can build on now)

| Capability | Backend maturity | Observer maturity | Frontend status | Stable? |
|-----------|-----------------|-------------------|-----------------|---------|
| Runtime summary stream | M3 | M4 — full read model + wire | ✅ Rendered | Yes |
| Chart-qualified world chunks | M2 | M4 — bounded projection | ✅ Rendered | Yes |
| Material surface deltas | M3 | M4 — V3 schema with local mana | ✅ Decoded, partially rendered | Yes |
| Material surface gate deltas | M3 | M4 — V3 schema | Decoded, not rendered | Yes |
| Explanation IR (material-surface loop) | M3 | M4 — typed claims with evidence | ✅ Rendered | Yes |
| Digest anchors | M3 | M4 — verified on every update | ✅ Verified | Yes |
| Protocol negotiation | M3 | M4 — version + capabilities | ✅ Working | Yes |
| Simulation control (pause/step/reset/seed) | M3 | M4 | ✅ Working | Yes |
| Localization (ru-RU, en-US) | M4 | M4 — locale-invariant state | ✅ Working | Yes |
| Observer stream hub + backpressure | M3 | M3 — capacity-1 used | Infrastructure only | Yes |

### Narrow but valid vertical slices

| Capability | What exists | What's narrow | Could expand when |
|-----------|------------|--------------|-------------------|
| Causal flow display | Runtime summary counts for physics, mana, resolution, actions | Aggregate counts only, not a real graph | Trace query read model added |
| Timeline history | 96-sample client-side FIFO | Not authoritative, no persistence | Historical query API |
| Chunk inspector | Single chunk detail panel | Shows only current summary fields | More per-chunk data projected |
| Explanation claims | Material-surface loop schemas 10-15 | Only one experiment type | New claim schemas registered |

### Blocked primarily by missing observer projections

| Capability | Runtime data exists | What's missing |
|-----------|-------------------|----------------|
| Causal trace graph | `CausalTraceStore` with flat vectors, ancestry | Observer query kind, wire encoding, TS decoder |
| Entity/agent inspection | Entity model in runtime | `EntitySummary` read model, wire encoding |
| Mana field spatial detail | Per-cell mana field | Spatial read model, per-cell projection |
| Resolution field detail | Full resolution field | Spatial read model, per-cell projection |
| Performance telemetry | Partial metrics | `PerformanceMetrics` wire encoding |
| Rendered explanation text | `DeterministicExplanationRenderer` in Rust | Wire transport or TS port |

### Blocked by simulation domain maturity

| Capability | Domain maturity | What's needed |
|-----------|----------------|---------------|
| Language inspection | M1 | Runtime coupling, read model, wire |
| Social network viz | M1 | Agent-local inference, lifecycle, read model |
| Concept evolution | M1 | State-dependent goals, learning, read model |
| Practice lineage display | M1 | Embodied execution, transmission, read model |
| Agent subjective knowledge | M1-M2 | Scene, memory, belief read model |
| Historical comparison | No persistence queries | Persistence + time-range API |
| Ecology/climate viz | M0 | Domain implementation |
| City infrastructure | M1 | Generated structures, flows |

---

## 3. Current Contracts Assessment

### Stable contracts (safe to build against)

| Contract | Location | Why stable |
|----------|----------|-----------|
| Observer protocol v1 wire format | `proto/causafera/observer/v1/` + wire crate | Versioned, breaking changes need v2 |
| `QueryKind` enum (3 values) | `causafera-observer-api/src/query.rs` | Additive — new kinds don't break existing |
| `StreamKind` enum (3 values) | `causafera-observer-api/src/stream.rs` | Same |
| `ObserverSnapshot` fields | `query.rs` | Additive |
| `SpatialChunkSummary` fields | `query.rs` | Additive |
| `MaterialSurfaceDelta` V3 schema | `query.rs` | Versioned |
| Explanation IR claim model | `causafera-explanation/src/ir.rs` | Typed, schema-identified |
| Tauri command interface | `main.rs` | Additive |
| Digest 32-byte format | Wire protocol | Foundational |

### Provisional contracts (may evolve)

| Contract | Why provisional |
|----------|----------------|
| 3-chunk demo configuration | `session_config()` hardcodes values |
| 64-delta bounded window | `MAX_MATERIAL_SURFACE_DELTAS` constant |
| Single stream with capacity 1 | Only `RUNTIME_STREAM_ID` used |
| Material-surface loop as only explanation | `observer_material_surface_loop_explanation()` |
| `RuntimeSummary` field set | Will grow with new domain capabilities |

### Where domain evolution will force frontend adaptation

| Evolution area | Impact | Can be absorbed by |
|---------------|--------|-------------------|
| New query kinds | New data types to decode and display | Additive protocol extension |
| New chunk summary fields | New values in existing structure | Additive fields |
| New explanation claim schemas | New claim types with new schema IDs | Claim rendering by schema ID (already works) |
| Entity data | Entirely new inspection surface | New view/panel |
| Multi-chart world | Chart identity already in coordinates | Navigation between charts |
| Larger active chunk sets | More chunks in same structure | Scalable spatial renderer |
| New stream kinds | New real-time data channels | New subscription handlers |

### How future domains can expose information without global redesign

The existing architecture is inherently extensible:

1. **New query kinds:** Add `QueryKind` variant → Rust read model → wire encode → TS decode → new component. Existing queries unaffected.

2. **New claim schemas:** Explanation already uses opaque `schemaId`. New schemas register automatically. Frontend can render unknown schemas generically (already has fallback in Rust renderer).

3. **New stream kinds:** `StreamKind` is extensible. New streams subscribe independently. Closed panels don't receive updates.

4. **New chunk fields:** Additive protobuf fields. Unknown fields are skipped by existing decoders.

5. **New entity types:** Entirely new query/response pair following existing patterns.

---

## 4. Candidate Early Implementation Scope

### What this scope aims to produce

- A strong, reusable frontend foundation (design system, component library, layout system)
- A credible and impressive primary observer experience
- Several complete workflows using real current data
- Honest capability handling
- Improved world/spatial observation
- Strong integration of existing Explanation and causal evidence
- Infrastructure that future domains can extend without complete redesign

### Tier 1 — Strong foundation (current data fully supports)

| Capability | Data source | Risk |
|-----------|------------|------|
| **Application shell and design system** | None needed — pure frontend | Low |
| **Runtime dashboard** | `RuntimeSummary` stream | Low |
| **World observation** | `WorldChunkSnapshot` query | Low (needs scalable renderer for future) |
| **Material surface inspection** | `MaterialSurfaceDelta` + `GateDelta` | Low |
| **Explanation viewer** | `ExplanationReport` query | Low |
| **Simulation controls** | Existing Tauri commands | Low |
| **Connection and capability states** | Existing `hasTauriTransport()` | Low |
| **Localization** | Existing i18n system | Low |
| **Digest identity display** | Existing summary fields | Low |

### Tier 2 — Extends current capabilities (moderate backend work)

| Capability | Required backend work | Risk |
|-----------|----------------------|------|
| **Causal trace browser** | Trace query read model + wire | Medium — store exists, query interface needed |
| **Rendered explanation text** | TS port of `DeterministicExplanationRenderer` or Rust→wire | Low-medium |
| **Multiple stream subscriptions** | ObserverStreamHub already supports this; need Tauri event channel | Medium |
| **Performance metrics panel** | Wire encoding for `PerformanceMetrics` | Low |
| **Richer timeline** | More stream data + possibly persistence | Medium |

### Tier 3 — Requires domain maturity (defer)

| Capability | Why defer |
|-----------|----------|
| Entity/agent deep inspection | Needs entity read model (not implemented) |
| Language browser | Language domain at M1 |
| Social network visualization | Social domain at M1 |
| Concept evolution tracking | Cognition at M1-M2 |
| Historical state comparison | No historical persistence queries |
| Subjective knowledge display | Cognition read models not available |
| Ecology/climate overlays | Domains at M0 |

### What would be fake showcase unsupported by real data

- Language trees or word origin visualizations (M1, no read model)
- Social network graphs (M1, no read model)
- Agent decision or belief inspection (M1-M2, no read model)
- Historical timeline with rollback (no persistence queries)
- Climate or ecology overlays (M0)
- City infrastructure maps (M1, no read model)
- Rendering based on generated textures or 3D game objects
- Any data substituted from demo/fixture sources (INV-039 violation)

### What should not create excessive redesign risk

- Investing heavily in 3D rendering infrastructure (visualization direction favors 2D/2.5D)
- Building an entity inspector before entity read models exist
- Creating dedicated views for every M0/M1 domain
- Hardcoding layout assumptions about specific domain combinations
- Using a framework that makes the protocol transport layer difficult to extend

---

## 5. Parts of an Early Frontend That Can Become Long-Lived

| Part | Why long-lived |
|------|---------------|
| Design system (tokens, colors, typography, spacing) | Visual identity persists |
| Component library (panels, tables, charts, controls) | Domain-independent |
| Application shell and navigation infrastructure | Extends with new views |
| Transport layer (`ObserverClient`, codec) | Protocol versioned |
| Session state management pattern | Extends with new data |
| Localization infrastructure | Grows with new strings |
| Capability-aware UI state system | All future domains need it |
| Responsive layout system | Application requirement |
| Accessibility infrastructure | Universal requirement |
| Build and dev tooling | Stable |

---

## 6. Parts That Must Remain Capability-Aware and Adaptable

| Part | Why adaptable |
|------|--------------|
| View/workspace registry | New views added as domains mature |
| Spatial renderer | Must scale from 3 chunks to thousands |
| Explanation claim renderer | New claim schemas will be registered |
| Data decoders | New wire types as protocol grows |
| Inspector panels | New entity types and data structures |
| Metric displays | New metrics as domains mature |
| Navigation structure | New top-level areas possible |

---

## 7. Unresolved Architectural Questions

| Question | Impact | Who decides |
|----------|--------|-------------|
| Should the world renderer use WebGPU, Canvas 2D, or SVG? | Performance at scale | Claude Opus (rendering technology) |
| How should multiple streams be transported? Tauri events or polling? | Real-time data delivery | Claude Opus + backend protocol work |
| Should explanation rendering happen in Rust or TypeScript? | Rendering pipeline | Claude Opus (may request either) |
| How should chart-qualified coordinates be visually represented? | Map presentation | Claude Opus (spatial design) |
| Should the app use a state management library? | Complexity management | Claude Opus (component architecture) |
| How should the app handle very large numbers of chunks? | Scalability | Claude Opus (rendering + virtualization) |
| Should causal traces be visualized as a graph, timeline, table, or hybrid? | Provenance presentation | Claude Opus (information design) |

---

## 8. Items Requiring Prototypes

- WebGPU or Canvas-based spatial renderer for chunk grids
- Causal trace graph navigation UI (once trace query exists)
- Explanation claim rendering with full evidence and confidence display
- Capability-aware view states (available → partial → unavailable)
- Dark terra incognita background treatment (performance + readability testing)
- Cartographic control interaction patterns

---

## 9. Items Requiring RFCs or ExecPlans

Per `PLANS.md`, the following require an ExecPlan before implementation:

- Observer protocol changes (new query kinds, stream types)
- Explanation architecture changes (new claim schemas, rendering pipeline)
- Persistence format changes (historical query access)

Claude Opus may propose these changes but cannot unilaterally implement them in the backend.
Frontend work that only consumes existing protocol data does not require an ExecPlan.
