# Frontend Redesign — Claude Opus Implementation Handoff

**Status:** Research material and technical orientation for Claude Opus.
Not an approved UI design. Every non-binding observation may be rejected.

**Audience:** The Claude Opus agent who will act as authoritative frontend director,
designer, and primary implementation agent.

**Companion documents:**

- [`docs/observer/frontend-architectural-groundwork.md`](frontend-architectural-groundwork.md) —
  binding product intent, visual direction, architectural constraints
- [`docs/observer/capability-maturity-map.md`](capability-maturity-map.md) —
  current capability matrix, data availability, candidate early scope

---

## 1. Repository Orientation

### Workspace structure

```text
Causafera/
├── apps/observer/             ← Tauri 2 desktop application
│   ├── src/                   ← React frontend (Vite + TypeScript)
│   ├── src-tauri/             ← Rust Tauri shell (commands, session)
│   ├── index.html
│   ├── vite.config.ts
│   └── package.json
├── packages/observer-protocol/ ← TypeScript observer protocol codec
│   └── src/index.ts
├── crates/                    ← 23 Rust crates
│   ├── causafera-observer-api/   ← Observer API types (queries, streams, snapshots)
│   ├── causafera-observer-wire/  ← Wire encoding/decoding (protobuf-like)
│   ├── causafera-runtime/        ← Deterministic simulation runtime
│   ├── causafera-explanation/    ← Explanation IR, claims, rendering
│   ├── causafera-types/          ← Core types (coords, ids, physics)
│   ├── causafera-domains/        ← Domain implementations (city, economy, mana, social, practices)
│   ├── causafera-world/          ← World geometry and spatial hierarchy
│   ├── causafera-lab/            ← Experiment runner
│   └── ... (20 more domain crates)
├── proto/causafera/observer/v1/  ← Protocol Buffer schema definitions (10 files)
├── docs/                         ← Extensive project documentation (~170 files)
├── plans/                        ← ExecPlans (active, draft, historical)
├── tests/integration/            ← Workspace-level integration tests
├── package.json                  ← pnpm workspace root
└── pnpm-workspace.yaml           ← apps/* + packages/*
```

### Frontend application entry points

| Path | Purpose | Status |
|------|---------|--------|
| `apps/observer/index.html` | HTML shell, loads Vite dev server | Stable infrastructure |
| `apps/observer/src/main.tsx` | React root render (`<App />` in StrictMode) | Stable infrastructure |
| `apps/observer/src/App.tsx` | Application shell, view switching, layout | Provisional — replaceable |
| `apps/observer/src/useObserverSession.ts` | Session state hook (connection, summary, world, history, explanation) | Reusable pattern, replaceable implementation |
| `apps/observer/src/observerClient.ts` | Tauri transport wrapper — encodes/decodes protocol bytes | Reusable |
| `apps/observer/src/i18n.ts` | Localization dictionaries (ru-RU, en-US) | Reusable pattern, content will grow |
| `apps/observer/src/styles.css` | Complete vanilla CSS design system (~1130 lines) | Provisional — likely replaced |

### Tauri shell (Rust side)

| Path | Purpose | Status |
|------|---------|--------|
| `apps/observer/src-tauri/src/main.rs` | Tauri command registration, `ObserverState` management | Stable infrastructure |
| `apps/observer/src-tauri/src/session.rs` | `ObserverSession` — runtime ownership, protocol handler, stream hub, query/advance/reset/analyze | Stable, well-tested |
| `apps/observer/src-tauri/Cargo.toml` | Dependencies: tauri, causafera-lab, -observer-api, -observer-wire, -runtime, -types | Stable |
| `apps/observer/src-tauri/tauri.conf.json` | Window config (1360×860, min 960×680), dev server port 1420 | Stable infrastructure |

### TypeScript observer-protocol package

| Path | Purpose | Status |
|------|---------|--------|
| `packages/observer-protocol/src/index.ts` | Complete v1 codec: types, encoders, decoders, Cursor class (~720 lines) | Stable, reusable |

### Rust observer crates

| Path | Purpose | Status |
|------|---------|--------|
| `crates/causafera-observer-api/src/lib.rs` | Re-exports `query` and `stream` modules | Stable |
| `crates/causafera-observer-api/src/query.rs` | `QueryKind`, `ObserverQuery`, `ObserverSnapshot`, `ObserverWorldSnapshot`, `ObserverChunkSummary`, `MaterialSurfaceDelta`, `MaterialSurfaceGateDelta` | Stable |
| `crates/causafera-observer-api/src/stream.rs` | `StreamKind`, `StreamScope`, `StreamEnvelope`, `ObserverStreamHub`, `DeliveryPolicy` | Stable |
| `crates/causafera-observer-wire/src/protocol.rs` | `ProtocolHandler`, `ConnectRequest`, `ConnectResponse`, all encode/decode functions | Stable |
| `crates/causafera-observer-wire/tests/protocol.rs` | Wire round-trip tests | Stable |

### Proto schema files

| Path | Key types |
|------|-----------|
| `proto/.../common.proto` | `SimulationTime`, `Digest`, `ChunkScope`, `QueryStatus` |
| `proto/.../control.proto` | `ConnectRequest`, `ConnectResponse`, `DisconnectRequest` |
| `proto/.../query.proto` | `QueryKind`, `QueryRequest`, `QueryResponse`, `RuntimeSummary`, `WorldChunkSnapshot`, `MaterialSurfaceDelta`, `MaterialSurfaceGateDelta` |
| `proto/.../stream.proto` | `StreamKind`, `DeliveryPolicy`, `SubscribeRequest`, `StreamHeader`, `StreamEnvelope` |
| `proto/.../spatial.proto` | `SpatialChunkSummary` |
| `proto/.../entity.proto` | `EntitySummary`, `NumericComponent` |
| `proto/.../causal.proto` | `CausalEventSummary` |
| `proto/.../explanation.proto` | `EvidenceState`, `Assessment`, `NumericClaimValue`, `ExplanationClaim`, `ExplanationFrame`, `ExplanationReport`, `RenderedExplanation` |
| `proto/.../language.proto` | `LexemeSummary`, `SubjectiveAssociation` |
| `proto/.../metrics.proto` | `PerformanceMetrics` |

---

## 2. Important Symbols

### TypeScript — `packages/observer-protocol/src/index.ts`

**Constants:**
- `OBSERVER_PROTOCOL_V1 = 1` — protocol version
- `MAX_MATERIAL_SURFACE_DELTAS = 64` — bounded window
- `MATERIAL_SURFACE_DELTA_SCHEMA_V3 = 3` — current delta schema

**Enums:**
- `QueryKind { RuntimeSummary=1, ExplanationIr=2, WorldChunks=3 }`
- `QueryStatus { Ok=1, InvalidRequest=2, Unsupported=3, NotAvailable=4 }`
- `StreamKind { RuntimeSummary=1, Explanation=2, Metrics=3 }`
- `EvidenceState { Supported=1, Unsupported=2, Unknown=3 }`
- `Assessment { Supported=1, Partial=2, Unsupported=3, Unknown=4 }`

**Key interfaces:**
- `RuntimeSummary` — 23 fields including ticks, digests, mana, population, actors, traces, events
- `WorldChunkSnapshot` — ticks, chunks[], materialSurfaceDeltas[], materialSurfaceGateDeltas[]
- `SpatialChunkSummary` — chartId, chunkX/Y/Z, elevation, roughness, mana, resolution, population, events, trace
- `MaterialSurfaceDelta` — chunk coords, before/after condition, mana, traces (optional fields for V3)
- `ExplanationReport` — experimentId, frames[], overallAssessment
- `ExplanationFrame` — checkpointTicks, claims[], overallAssessment
- `ExplanationClaim` — schemaId, value (scalar/range/ratio), confidence, evidenceTraceIds, comparison, evidenceState
- `StreamEnvelope` — header (streamId, schemaVersion, sequenceNumber, simulationTime, digests, isSnapshot), kind, payload
- `ConnectRequest` / `ConnectResponse`

**Encoders:** `encodeConnectRequest`, `encodeQuery`, `encodeRuntimeSummaryQuery`

**Decoders:** `decodeConnectResponse`, `decodeQueryResponse`, `decodeRuntimeSummary`, `decodeWorldChunkSnapshot`, `decodeStreamEnvelope`, `decodeExplanationReport`

**Utility:** `digestHex(value, length?)` — hex string from digest bytes

### TypeScript — `apps/observer/src/useObserverSession.ts`

- `ObserverLocale = "ru-RU" | "en-US"`
- `ConnectionState = "connecting" | "connected" | "unavailable" | "error"`
- `PlaybackRate = 1 | 4 | 16`
- `TimelineSample { ticks, mana, traces, physicalEvents }` — all bigint
- `ObserverSessionModel` — full session interface returned by the hook
- `TIMELINE_CAPACITY = 96` — client-side FIFO size
- `chunkKey(chunk)` → `"chartId:x:y:z"` string

### TypeScript — `apps/observer/src/observerClient.ts`

- `ObserverClient` class — wraps Tauri invoke calls
  - `connect(locale)` → `ConnectResponse`
  - `openRuntimeStream()` → `RuntimeUpdate`
  - `advance(ticks)` → `RuntimeUpdate`
  - `reset(seed)` → `RuntimeUpdate`
  - `world()` → `WorldChunkSnapshot`
  - `analyze()` → `ExplanationReport`
- `hasTauriTransport()` — checks `window.__TAURI__`
- `RuntimeUpdate { summary, sequenceNumber, isSnapshot }`

### Rust — Tauri commands (`apps/observer/src-tauri/src/main.rs`)

Six registered commands: `observer_connect`, `observer_open_stream`, `observer_advance`, `observer_query`, `observer_analyze`, `observer_reset`. State held in `Arc<Mutex<ObserverSession>>`.

### Rust — `ObserverSession` (`apps/observer/src-tauri/src/session.rs`)

- `ObserverSession { runtime: Runtime, protocol: ProtocolHandler, streams: ObserverStreamHub }`
- `MAX_ADVANCE_TICKS = 64` — bounded tick advancement
- `DEFAULT_ACTORS = 8`, `DEFAULT_SENSORS = 2`, `DEFAULT_POPULATION = 512`
- Methods: `new(seed)`, `connect(&[u8])`, `query(&[u8])`, `open_runtime_stream()`, `advance(ticks)`, `analyze(&[u8])`, `reset(seed)`
- 4 unit tests covering negotiation, world queries, locale invariance, advance bounds

### Rust — Observer API (`crates/causafera-observer-api/src/query.rs`)

- `QueryKind { RuntimeSummary, ExplanationIr, WorldChunks }`
- `ObserverQuery { request_id, protocol_version, kind, scope, payload }`
- `ObserverSnapshot` — mirrors TypeScript `RuntimeSummary`
- `ObserverWorldSnapshot { time, chunks: Vec<ObserverChunkSummary>, material_surface_deltas, gate_deltas, schema_version }`

### Rust — Explanation (`crates/causafera-explanation/src/ir.rs`)

- `ExplanationClaimSchemaId`, `ClaimConfidence`, `ComparisonCohortId` — typed wrappers
- `NumericClaimValue { Scalar, Range, Ratio }`
- `ComparisonContext { None, MatchedCohort, Counterfactual }`
- `ClaimEvidenceState { Supported, Unsupported, Unknown }`
- `ExplanationClaim`, `ExplanationFrame`, `ExplanationReport`
- `MaterialSurfaceLoopClaim`, `MaterialSurfaceLocalManaTransitionClaim`

---

## 3. Key Code Excerpts

### Application entry and view switching

```tsx
// apps/observer/src/App.tsx — lines 15-30
type ViewId = "world" | "causality" | "explanation";

export function App() {
  const [view, setView] = useState<ViewId>("world");
  const session = useObserverSession(view === "world");
  const copy = copyFor(session.locale);
  // ... view renders conditionally based on `view` state
}
```

**Why it matters:** Simple state-driven view switching with no router. Claude Opus may replace
this entirely with a workspace/tab/navigation system.
**Status:** Provisional — replaceable.

### Observer session model

```tsx
// apps/observer/src/useObserverSession.ts — lines 22-40
export interface ObserverSessionModel {
  connection: ConnectionState;
  summary?: RuntimeSummary;
  world?: WorldChunkSnapshot;
  history: TimelineSample[];
  explanation?: ExplanationReport;
  error?: string;
  isPlaying: boolean;
  isAnalyzing: boolean;
  locale: ObserverLocale;
  playbackRate: PlaybackRate;
  step(): Promise<void>;
  togglePlayback(): void;
  reset(seed: number): Promise<void>;
  analyze(): Promise<void>;
  setLocale(locale: ObserverLocale): void;
  setPlaybackRate(rate: PlaybackRate): void;
  refreshWorld(): Promise<void>;
}
```

**Why it matters:** Defines the complete current session contract. All data available to the
frontend flows through this interface.
**Status:** Reusable pattern. Interface will expand as capabilities grow.

### Tauri transport bridge

```tsx
// apps/observer/src/observerClient.ts — lines 117-123
private async invokeBytes(command: string, args?: Record<string, unknown>): Promise<Uint8Array> {
  const invoke = window.__TAURI__?.core.invoke;
  if (invoke === undefined) throw new Error("Tauri observer bridge is unavailable");
  const serializedArgs = args === undefined ? undefined : mapByteArguments(args);
  const response = await invoke<number[] | Uint8Array>(command, serializedArgs);
  return response instanceof Uint8Array ? response : Uint8Array.from(response);
}
```

**Why it matters:** All data crosses the Rust↔JS boundary as raw bytes through this single
method. The byte arrays are then decoded by the observer-protocol package.
**Status:** Stable, reusable.

### Disconnected behavior

```tsx
// apps/observer/src/useObserverSession.ts — lines 80-85
if (!hasTauriTransport()) {
  setConnection("unavailable");
  return undefined;
}
```

**Why it matters:** The frontend explicitly degrades when Tauri is unavailable. It never
substitutes demonstration data. This is an architectural requirement (INV-039).
**Status:** Architecturally mandatory pattern.

### Rust session — refresh protocol and publish

```rust
// apps/observer/src-tauri/src/session.rs — lines 81-105
fn refresh_protocol(&mut self) -> Result<(), SessionError> {
    let snapshot = self.runtime.snapshot()?;
    let world = self.runtime.observer_world_snapshot()?;
    self.protocol.set_runtime_snapshot(&snapshot.observer_snapshot());
    self.protocol.set_world_snapshot(&world);
    Ok(())
}

fn publish_runtime(&mut self, is_snapshot: bool) -> Result<Vec<u8>, SessionError> {
    let snapshot = self.runtime.snapshot()?.observer_snapshot();
    self.streams.publish(
        RUNTIME_STREAM_ID, snapshot.time, snapshot.physical_digest,
        snapshot.history_digest, is_snapshot,
        encode_observer_snapshot(&snapshot),
    )?;
    let envelope = self.streams.pop(RUNTIME_STREAM_ID)?
        .ok_or(SessionError::MissingStreamEnvelope)?;
    Ok(encode_stream_envelope(&envelope))
}
```

**Why it matters:** Shows how runtime state becomes observer bytes. `Runtime::snapshot()` and
`Runtime::observer_world_snapshot()` are the only read paths. Protocol handler and stream hub
manage encoding and delivery.
**Status:** Stable infrastructure.

### Digest verification in client

```tsx
// apps/observer/src/observerClient.ts — lines 95-103
if (
  digestHex(summary.physicalDigest) !== digestHex(envelope.header.physicalDigest) ||
  digestHex(summary.historyDigest) !== digestHex(envelope.header.historyDigest)
) {
  throw new Error("stream digest anchor does not match runtime payload");
}
```

**Why it matters:** The client verifies digest anchors on every runtime update. This is not
optional validation — it ensures stream consistency.
**Status:** Architecturally mandatory.

---

## 4. Data-Flow Summaries

### Runtime summary stream

```text
Runtime::tick()
  → Runtime::snapshot() → RuntimeSnapshot → .observer_snapshot() → ObserverSnapshot
    → ProtocolHandler::set_runtime_snapshot()
    → ObserverStreamHub::publish() → StreamEnvelope
      → encode_stream_envelope() → Vec<u8>
        → Tauri command observer_advance / observer_open_stream
          → window.__TAURI__.core.invoke → Uint8Array
            → decodeStreamEnvelope() → StreamEnvelope
              → decodeRuntimeSummary() → RuntimeSummary
                → useObserverSession state update
                  → React component props
```

### World chunk query

```text
Runtime::observer_world_snapshot() → ObserverWorldSnapshot
  → ProtocolHandler::set_world_snapshot()
  → ProtocolHandler::handle_query(WorldChunks) → encoded QueryResponse
    → Tauri command observer_query
      → decodeQueryResponse() → payload bytes
        → decodeWorldChunkSnapshot() → WorldChunkSnapshot
          → WorldViewport component
```

### Explanation analysis

```text
Runtime::observer_material_surface_loop_explanation() → ExplanationReport (Rust)
  → ProtocolHandler::set_explanation_report()
  → ProtocolHandler::handle_query(ExplanationIr) → encoded QueryResponse
    → Tauri command observer_analyze (async/blocking)
      → decodeExplanationReport() → ExplanationReport (TS)
        → ExplanationPanel component
```

### Session control (connect/reset)

```text
encodeConnectRequest({versions:[1], locale}) → bytes
  → Tauri observer_connect
    → ObserverSession::connect() → ProtocolHandler::negotiate()
      → encode_connect_response() → bytes
        → decodeConnectResponse() → ConnectResponse
```

### Where future capabilities enter

New data types would enter at `Runtime::snapshot()` or new `Runtime::observer_*()` methods →
new fields in `ObserverSnapshot` or new query kinds → wire encoding in `causafera-observer-wire`
→ new proto messages → new TypeScript decoder in `observer-protocol` → new session state
fields → new components.

---

## 5. Current Capability Matrix

| Data | Frontend availability | Backend availability | Blocker |
|------|----------------------|---------------------|---------|
| Runtime summary (ticks, digests, mana, population, actors, traces, events) | ✅ Full | ✅ Full | — |
| World chunk projection (chart-qualified, elevation, roughness, mana, resolution, population, events) | ✅ Full | ✅ Full (bounded 3-chunk demo config) | — |
| Material surface deltas (condition transitions, mana, traces) | ✅ Full | ✅ Full (bounded window of 64) | — |
| Material surface gate deltas (local mana coupling) | ✅ Decoded, not rendered | ✅ Full | No UI component |
| Explanation IR (typed claims, evidence, confidence, comparison) | ✅ Full | ✅ Full (bounded experiment) | — |
| Deterministic explanation text rendering | ❌ Not in frontend | ✅ Rust `DeterministicExplanationRenderer` | Needs TS port or Rust→wire path |
| Causal trace ancestry/graph | ❌ Not exposed | Partial — `CausalTraceStore` exists, read-only views | Needs observer query + wire encoding |
| Entity snapshots / agent state | ❌ Not exposed | `EntitySummary` proto exists, not implemented | Needs read model + wire |
| Population aggregates | Only total count | Only totals | Needs domain read model |
| Language data | ❌ Not exposed | `LexemeSummary` proto exists, not implemented | M1 domain maturity |
| Social network data | ❌ Not exposed | M1 contracts exist | Needs read model |
| Mana field visualization | Only chunk-level totals | Per-cell field exists in runtime | Needs spatial read model |
| Resolution field visualization | Chunk-level relevance+level | Full field exists | Needs spatial read model |
| Historical state comparison | ❌ No history access | No historical storage | Needs persistence + query |
| Performance metrics | ❌ Not exposed | `PerformanceMetrics` proto exists | Needs telemetry read model |
| Subjective/agent knowledge | ❌ Not exposed | Cognition crate exists (M1-M2) | Deep domain maturity needed |
| Objective vs subjective comparison | ❌ Not exposed | Architectural distinction exists | Needs read models for both |
| Practice lineages | ❌ Not exposed | M1 contracts | Needs read model |
| Concept evolution | ❌ Not exposed | M1 contracts | Needs read model |

### Classification

**Fully supported for frontend now:**
- Runtime summary stream with digest anchors
- Chart-qualified world chunk projection
- Material surface deltas (V3 schema with local mana)
- Comparative explanation experiment with typed IR

**Partially supported (narrow vertical slice):**
- Material surface gate deltas (decoded, not rendered)
- Causal flow aggregate (uses runtime summary counts, not graph)

**Blocked primarily by missing observer projections/protocol work:**
- Causal trace graph (store exists, no query/wire path)
- Entity inspection (proto defined, no implementation)
- Mana field spatial detail (runtime has data, no projection)
- Resolution field spatial detail
- Performance metrics

**Blocked by simulation domain maturity:**
- Language inspection (M1)
- Social network visualization (M1)
- Concept evolution tracking (M1)
- Practice lineage display (M1)
- Agent subjective knowledge (M1-M2)
- Historical comparison (no persistence)

---

## 6. Relevant Invariants Checklist

| ID | Summary | Source | Impact on frontend |
|----|---------|--------|--------------------|
| INV-006 | No privileged UI language | `docs/architecture/invariants.md` | Locale is presentation only; cannot change state hash |
| INV-007 | Changing locale cannot change state hash | Same | Verified by test `locale_does_not_change_session_digests` |
| INV-011 | LLMs are non-authoritative | Same | Optional LLM wording forbidden until terminal gate |
| INV-012 | Explanation is non-authoritative | Same | Explanation never modifies state |
| INV-013 | Observer classifications cannot feed back | Same | UI labels/categories are read-only |
| INV-021 | UI is an observer | Same | Desktop app never reads internal storage directly |
| INV-022 | Rendering is not state | Same | Visual representation is not simulation truth |
| INV-026 | Explanations expose confidence + provenance | Same | Every claim must include evidence state |
| INV-036 | Spatial coordinate scope explicit | Same | Chart-qualified coordinates; no seamless global map |
| INV-037 | Geometry ≠ containment ≠ resolution | Same | Separate hierarchies |
| INV-038 | Digests are identities, not metrics | Same | Never use digest-byte distance as similarity |
| INV-039 | Production state requires causal init | Same | Never substitute demo/fixture data |
| INV-042 | Architecture remains modular | Same | Keep modules cohesive |

**Authoritative sources:**
- `docs/architecture/invariants.md` — 42 hard invariants
- `docs/architecture/observer.md` — observer pipeline
- `docs/architecture/protocol.md` — protocol boundaries
- `docs/architecture/detailed-development-rebaseline.md` — maturity model, priority order
- `docs/observer/backpressure.md` — delivery policies

---

## 7. Current Implementation Inventory

### Reusable infrastructure

| Item | Path | Why reusable |
|------|------|-------------|
| Tauri command registration | `src-tauri/src/main.rs` | Clean command pattern, extend with new commands |
| `ObserverSession` | `src-tauri/src/session.rs` | Well-tested session management, protocol handler |
| `ObserverClient` class | `src/observerClient.ts` | Clean transport wrapper |
| `@causafera/observer-protocol` | `packages/observer-protocol/src/index.ts` | Complete v1 codec with Cursor class |
| `hasTauriTransport()` | `src/observerClient.ts` | Disconnected detection |
| i18n pattern | `src/i18n.ts` | Dictionary-based locale system |
| `useObserverSession` session pattern | `src/useObserverSession.ts` | Session state management pattern |
| Digest verification | `observerClient.ts:95-103` | Stream consistency check |
| pnpm workspace config | `package.json`, `pnpm-workspace.yaml` | Build infrastructure |
| Vite + React setup | `vite.config.ts`, `tsconfig.json` | Build tooling |
| Session tests | `src-tauri/src/session.rs` (4 tests) | Protocol and session verification |
| Wire round-trip tests | `crates/causafera-observer-wire/tests/protocol.rs` | Wire correctness |

### Provisional — likely replaced

| Item | Path | Why provisional |
|------|------|----------------|
| `App.tsx` layout | `src/App.tsx` | Fixed sidebar + workspace layout; simple view state |
| `styles.css` | `src/styles.css` | Complete but basic dark scientific dashboard; 1130 lines |
| `WorldViewport` | `src/components/WorldViewport.tsx` | Grid of chunk cells for 3-chunk demo; not scalable |
| `CausalFlow` | `src/components/CausalFlow.tsx` | Static numeric list; not a real causal graph |
| `TimelinePanel` | `src/components/TimelinePanel.tsx` | SVG line chart; 96-sample FIFO |
| `InspectorPanel` | `src/components/InspectorPanel.tsx` | Single chunk details panel |
| `MetricCard` | `src/components/MetricCard.tsx` | Simple KPI card |
| `ExplanationPanel` | `src/components/ExplanationPanel.tsx` | Claim list for bounded experiment |
| `ExplanationClaimRow` | `src/components/ExplanationClaimRow.tsx` | Individual claim row |
| `SimulationControls` | `src/components/SimulationControls.tsx` | Play/pause/step/reset/seed |
| `ConnectionStatus` | `src/components/ConnectionStatus.tsx` | Status dot indicator |
| `DefinitionRow` | `src/components/DefinitionRow.tsx` | Key-value helper |
| View switching mechanism | `App.tsx` line 18 | `useState<ViewId>` with 3 views |
| Responsive breakpoints | `styles.css` lines 946-1129 | Basic 1080/820/560px breakpoints |

### Technical debt

- **No frontend tests.** Zero `.test.ts` files exist. The Rust session has 4 good tests.
- **No client-side routing.** View switching is a single `useState`. No URL state, no deep linking.
- **No component library or design tokens.** CSS variables exist but no systematic token layer.
- **Timeline is not authoritative.** 96-sample FIFO is presentation state only.
- **World viewport is not scalable.** DOM-based chunk grid cannot handle more than ~9 chunks.
- **No WebGPU or canvas rendering.** Everything is DOM-based.
- **No error recovery.** Connection error is terminal until page reload.
- **No streaming subscriptions.** World data uses polling (request/response on each advance).
- **Explanation only for material-surface loop.** Hardcoded to single experiment type.
- **No keyboard navigation for views.** Chunks have keyboard support; views do not.

---

## 8. Technical Dependency Analysis

For likely future frontend capabilities:

### Causal trace exploration
- **Needs:** Read model in `causafera-observer-api` for trace queries, wire encoding, proto schema for trace graph, TS decoder, new query kind
- **Available:** `CausalTraceStore` in `causafera-core` with flat vectors and ancestry traversal

### Entity inspection
- **Needs:** `EntitySummary` read model implementation, wire encoding, TS decoder
- **Available:** Proto schema exists (`entity.proto`), entity model in runtime

### Mana field visualization
- **Needs:** Per-cell spatial read model, wire encoding, likely WebGPU renderer for density
- **Available:** Per-cell mana field in runtime, chunk-level totals already projected

### Historical comparison
- **Needs:** Persistence implementation for historical snapshots, time-range query API
- **Available:** Persistence crate exists but historical query not implemented

### Rich explanation rendering
- **Needs:** Either TS port of `DeterministicExplanationRenderer` or rendering in Rust + wire transport of rendered text
- **Available:** Rust renderer exists with English/Russian templates, `RenderedExplanation` proto defined

### Performance dashboard
- **Needs:** Telemetry read model, `PerformanceMetrics` wire encoding, TS decoder
- **Available:** Proto defined, metrics partially collected

### Streaming subscriptions
- **Needs:** Full `SubscribeRequest` → Tauri event channel (not request/response)
- **Available:** `ObserverStreamHub` supports multiple streams with delivery policies; currently only one stream used

---

## 9. Anticipated Change Map

### Files to inspect first

1. `apps/observer/src/App.tsx` — current shell, will be replaced
2. `apps/observer/src/styles.css` — current design, will be replaced
3. `apps/observer/src/useObserverSession.ts` — session model to extend
4. `packages/observer-protocol/src/index.ts` — codec to extend
5. `apps/observer/src-tauri/src/session.rs` — Rust session to extend

### Files likely to modify

- `packages/observer-protocol/src/index.ts` — new decoders for new data types
- `apps/observer/src/useObserverSession.ts` — new state fields, new operations
- `apps/observer/src/observerClient.ts` — new query methods
- `apps/observer/src/i18n.ts` — many new translation strings
- `apps/observer/src-tauri/src/session.rs` — new query handlers
- `apps/observer/src-tauri/src/main.rs` — possibly new Tauri commands

### Files likely to replace entirely

- `apps/observer/src/App.tsx`
- `apps/observer/src/styles.css`
- All `src/components/*.tsx` (11 component files)

### New modules likely needed

- Design system / tokens (CSS or CSS-in-JS)
- Component library (panels, inspectors, charts, maps, tables)
- State management (possibly Zustand or similar if React state becomes complex)
- Workspace/view management system
- WebGPU or Canvas renderer for spatial visualization
- Rich table / data grid component
- Causal graph visualization
- Keyboard navigation system

### Protocol surfaces that may need extension

- `proto/.../query.proto` — new query kinds
- `proto/.../entity.proto` — implementation of entity snapshots
- `proto/.../causal.proto` — trace query responses
- `proto/.../metrics.proto` — telemetry data
- `crates/causafera-observer-api/src/query.rs` — new snapshot types
- `crates/causafera-observer-wire/src/protocol.rs` — new encoders

**Uncertainty:** The exact protocol extensions depend on which capabilities Claude Opus
prioritizes. The wire layer is straightforward to extend following existing patterns.

---

## 10. Recommended Reading Order

### Essential — read before designing

1. **This handoff** (`docs/observer/frontend-redesign-handoff.md`)
2. **Product intent and visual direction** (`docs/observer/frontend-architectural-groundwork.md`)
3. **Capability maturity map** (`docs/observer/capability-maturity-map.md`)
4. **Architecture invariants** (`docs/architecture/invariants.md`) — 42 binding constraints
5. **Observer architecture** (`docs/architecture/observer.md`) — pipeline, separation of concerns
6. **Detailed development rebaseline** (`docs/architecture/detailed-development-rebaseline.md`) — maturity model M0-M5, priority order
7. **Observer protocol** (`docs/architecture/protocol.md`) — protocol boundaries, versioning
8. **Current application entry** (`apps/observer/src/App.tsx`)
9. **Session hook** (`apps/observer/src/useObserverSession.ts`)
10. **Observer client** (`apps/observer/src/observerClient.ts`)
11. **TypeScript protocol codec** (`packages/observer-protocol/src/index.ts`)
12. **Rust session** (`apps/observer/src-tauri/src/session.rs`)

### Useful — read when making specific decisions

- `docs/observer/protocol.md` — wire format details, delta schema
- `docs/observer/snapshots.md` — stream properties, scope management
- `docs/observer/backpressure.md` — delivery policies
- `docs/explanation/architecture.md` — explanation pipeline
- `docs/explanation/explanation-ir.md` — IR structure, perspectives
- `docs/explanation/deterministic-rendering.md` — rendering approach
- `docs/ui/observer-application.md` — Phase 26 application facts
- `docs/ui/views.md` — delivered and planned views
- `docs/ui/map-perspectives.md` — map perspective types
- `docs/architecture/provenance.md` — causal event model
- `docs/architecture/performance.md` — performance philosophy, benchmark states
- `apps/observer/src/styles.css` — current design (if adapting rather than replacing)

### Consult when modifying related capability

- `docs/ontology/domain-coverage-matrix.md` — all domain maturity
- `docs/roadmap/roadmap.md` — phase history and current program
- `docs/architecture/cognition-rebaseline.md` — cognitive architecture constraints
- `docs/explanation/confidence.md` — confidence representation
- `docs/explanation/causal-summaries.md` — trace summaries
- `docs/world/spatial-hierarchy.md` — geographic hierarchy
- `docs/rfc/RFC-GEO-002.md` — charted 2.5D planetary surface
- `AGENTS.md` — agent rules (must follow)

### Historical context only

- `docs/roadmap/roadmap.md` Phases 0-26 — completed foundation, not current guidance
- `plans/history/` — completed plans, provenance only
- `docs/development/rebaseline-report.md` — historical rebaseline record

---

## 11. Open Decisions for Claude Opus

Claude Opus has full authority over these decisions:

- Final visual language and design system
- Exact interpretation of terra incognita and cartographic motifs
- Information architecture and navigation structure
- Workspace model (tabs, panels, secondary windows)
- Concrete responsive behavior at different window sizes
- Component architecture (library, composition patterns)
- Rendering technology (DOM, Canvas, WebGPU, hybrid)
- Interaction patterns (keyboard, mouse, accessibility)
- Degree of current frontend replacement (partial or complete)
- Visual representation of capability maturity states
- How immature/unavailable capabilities are presented
- Concrete first implementation plan and sequencing
- State management approach (React state, Zustand, signals, etc.)
- Whether to use a CSS framework, design token system, or vanilla CSS
- Typography selection
- Color system design
- Motion and transition design
- How Explanation claims are visually presented
- How spatial data is rendered at scale
- How causal provenance is navigated

---

## 12. Decisions Requiring Broader Project Work

Claude Opus must not make these decisions purely inside the frontend:

| Decision | Requires | Authority |
|----------|----------|-----------|
| New observer query kinds | Observer API + wire encoding + proto schema | ExecPlan if observer protocol changes |
| New stream subscriptions | Stream hub extension + Tauri event channel | Protocol change |
| Entity snapshot implementation | Read model in observer-api + runtime projection | Protocol change |
| Causal trace query API | Trace store query interface + wire encoding | Protocol change |
| Historical state access | Persistence implementation | Persistence changes |
| Mana field spatial projection | New read model in runtime | Observer contract |
| New Explanation claim schemas | `causafera-explanation` changes | Explanation changes |
| Agent subjective state access | Cognition read model | Domain maturity + observer contract |
| Performance telemetry | Metrics collection + wire encoding | Observer contract |
| Demo/fixture data substitution | **Forbidden** by INV-039 | Architecturally mandatory prohibition |
| LLM-generated text | **Forbidden** until terminal gate | INV-011 + rebaseline terminal gate |
| Semantic labels as state | **Forbidden** by INV-006 | Architecturally mandatory prohibition |

Claude Opus may **request** legitimate observer, protocol, or backend changes. Such requests
should clearly specify what read model, query kind, or wire encoding is needed. They should not
be blocked merely because the current protocol is narrow.

---

## 13. Validation Commands

```bash
# TypeScript type checking
pnpm typecheck

# Linting
pnpm lint

# Build the TypeScript packages
pnpm build

# Run the desktop app in dev mode
pnpm --dir apps/observer desktop

# Alternative native Wayland
pnpm --dir apps/observer desktop:raw

# Rust workspace check
cargo check --workspace

# Rust tests
cargo test --workspace

# Specific session tests
cargo test -p causafera-observer-tauri

# Wire protocol tests
cargo test -p causafera-observer-wire

# Integration tests
cargo test --test determinism --test explanation_ir --test typed_ids

# Documentation formatting (if markdownlint available)
# No dedicated doc linter configured in the repository
```

---

## 14. Claude Opus Implementation Brief

**You are authoritative for the Causafera observer frontend.**

The repository is authoritative for simulation semantics and architectural boundaries.
This handoff is research material, not an approved UI design. Every non-binding design
observation may be rejected.

Key facts:

1. The current observer frontend is an early bounded implementation (Phase 26). You may
   replace it substantially or completely where that produces a better result.

2. The user's visual and product intent is documented in
   `docs/observer/frontend-architectural-groundwork.md`. That intent must be preserved.
   Its exact implementation is your decision.

3. The current frontend has real working data for: runtime summaries, chart-qualified
   world chunk projections, material surface deltas, and comparative explanation IR.
   These are genuine simulation outputs, not demo data.

4. Many capabilities are documented but not yet implemented in the observer layer.
   See the capability matrix in `docs/observer/capability-maturity-map.md`. Do not
   design against imaginary capabilities. Represent immature capabilities honestly.

5. You may request legitimate observer, protocol, or backend changes rather than
   contorting the UI around temporary limitations. Specify what you need clearly.

6. You must not violate authoritative boundaries (invariants) for presentation
   convenience. Key prohibitions: no demo data substitution (INV-039), no semantic
   state from UI labels (INV-006), no LLM wording (INV-011 + terminal gate),
   observer is read-only (INV-013, INV-021).

7. Real working workflows using current data should be prioritized over fictional
   domain screens. An honest, impressive observer with 4 real capabilities is better
   than 10 empty placeholder views.

8. Visual and runtime verification is mandatory. Test in the Tauri desktop shell.
   Validation commands are listed above.

9. The `@causafera/observer-protocol` package and `ObserverSession` Rust code are
   stable infrastructure. Extend them; avoid unnecessary rewrites.

10. The creative direction (midnight cartography + scientific instrumentation +
    restrained medieval fantasy) is binding product intent. How you interpret,
    refine, or implement it is your decision. Read the full vision in the
    architectural groundwork document.
