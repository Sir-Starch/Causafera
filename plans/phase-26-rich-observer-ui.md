# Phase 26 — Rich Observer UI

## Goal

Deliver a visually polished, usable Tauri + React observer that runs the real bounded Ontopolis runtime, negotiates observer protocol v1, receives runtime and world data only as versioned Protocol Buffer payloads, consumes bounded stream envelopes, and renders typed Explanation IR without introducing a UI mutation path into authoritative state.

## Context

Phase 24 produced the first executable long-run simulation. The post-Phase-24 depth plan closed its causal loops and added typed Explanation IR. The observer transport plan then delivered read-only runtime projection, query/response encoding, bounded streaming, deterministic localized explanation rendering, and locale-independence tests. `apps/observer` is still a static Phase 0 shell with disabled controls and no simulation connection.

## Relevant invariants

- INV-001 and INV-002: agent knowledge remains separate from objective observer data.
- INV-006 and INV-007: UI language is presentation state and cannot affect authoritative hashes.
- INV-012 and INV-013: Explanation and observer classifications are read-only.
- INV-021 and INV-022: UI and rendering are observers, never simulation state.
- INV-027 and INV-034: no authoritative identity or objective biological internals are exposed as subjective cognition.
- INV-036 and INV-037: chunk coordinates are chart-qualified; geometry, containment, and causal resolution remain separate.

## Ontology domains affected

Observer-only projections of geography, terrain, mana, causal resolution, biology/population aggregates, causal activity, and experiment analytics. No new authoritative domain semantics are introduced.

## Causal carriers affected

None. The UI reads numeric derived projections of existing carrier effects and provenance counts. It does not register, modify, or classify causal carriers.

## Relevant documents

`docs/architecture/{invariants,observer,protocol,performance}.md`, `docs/observer/*.md`, `docs/ui/*.md`, `docs/explanation/{architecture,explanation-ir,deterministic-rendering,localization}.md`, `docs/rfc/RFC-EXPLAIN-001.md`, ADR-004, ADR-005, and ADR-006.

## Current state

The React application contains only inline-style placeholders. The Tauri shell has no commands and is not buildable from the root Cargo workspace. Observer v1 supports runtime summary and Explanation IR queries plus stream envelopes, but the UI package only decodes a subset of runtime summary fields and has no negotiation, response, stream, spatial, or Explanation IR codecs. The runtime does not yet expose a bounded chart-qualified chunk projection.

## Proposed architecture

```text
Runtime / ExperimentRunner
    ↓ read-only projection
ObserverSnapshot + WorldChunkSummary + ExplanationReport
    ↓ canonical protobuf v1
ProtocolHandler / ObserverStreamHub
    ↓ byte-only Tauri commands
ObserverClient
    ↓ decoded view models
React research console
```

Execution controls are limited to session lifecycle and scheduler progression (`reset`, `step`, and bounded tick batches). They change execution parameters, not simulation content. All simulation data returned to React is encoded through observer v1. Pausing is a client decision to stop requesting tick batches. The default session uses explicit causal bootstrap parameters rather than demo residents or static sample data.

## Primitive vs emergent review

Chart identity, chunk coordinates, elevation range, roughness, numeric mana intensity, resolution relevance/level, population count, activity count, trace references, digests, time, and confidence are observer projection primitives. Cities, occupations, species, rituals, spells, social meanings, and narrative event names are not inferred or displayed. Human labels and colors are non-authoritative rendering metadata.

## Non-goals

- No direct runtime storage access from TypeScript.
- No entity-per-DOM rendering or unbounded world dataset.
- No invented cities, residents, languages, histories, or explanations.
- No LLM narrative surface.
- No mutation of authoritative world content from UI.
- No claim that the current bounded chart is a full planetary visualization.

## Implementation stages

1. Extend observer v1 with a bounded chart-qualified world-chunk query and complete TypeScript codecs for negotiation, responses, streams, runtime fields, spatial summaries, and Explanation IR.
2. Add runtime-side world projection that derives terrain bounds, roughness, mana, resolution, population, activity, and trace anchors without exposing mutable storage.
3. Implement a Tauri observer session with causal bootstrap, protocol negotiation, query dispatch, bounded latest-state stream delivery, reset/step/tick-batch controls, and replay-verified experiment analysis.
4. Build a typed React observer client and session hook that isolates transport, connection, timeline buffering, locale, play cadence, query cadence, and errors.
5. Replace the placeholder shell with a responsive research-console UI: navigation, restrained status chrome, world chunk map, metric rail, causal flow, timeline, inspector, and Explanation IR panel. Use a clean editorial dark theme with no sci-fi ornament, neon effects, decorative grids, or fictional instrumentation.
6. Add deterministic English/Russian UI rendering metadata while proving locale changes do not alter physical or history digests.
7. Verify Rust, protobuf, TypeScript, production build, visual behavior, accessibility basics, responsive layout, and browser console health; record performance diagnostics and documentation.

## Verification

- `protoc` validates all observer v1 schemas.
- Observer API/wire/runtime/session unit and integration tests pass.
- Same seed and equal tick progression produce identical stream payloads and digests.
- Locale changes alter only UI text.
- `cargo test --workspace --all-features` and strict clippy pass.
- Frontend typecheck and production build pass.
- Browser verification checks load, console errors, key panels, keyboard controls, and desktop/mobile layouts.

## Benchmark plan

Measure release-mode session startup, one-tick stream projection, world query encoding, and bounded 192-tick analysis. Record diagnostics without CI wall-time thresholds or scale claims. Retain the existing observer-off/idle/normal/heavy benchmark.

## Determinism impact

The runtime seed and tick count fully determine authoritative state. UI cadence may change when the runtime advances but cannot change a given tick's result. Locale, selected panel, hover state, timeline buffer, animations, and transport queue state remain outside authoritative digests.

## Memory impact

World projections are bounded by active chunks. The Rust stream capacity is one for latest-state-wins runtime summaries. The client timeline uses a fixed-size FIFO. No observer queue or chart history grows without a cap.

## Observer impact

Adds the first complete external consumer of observer v1 and a spatial query useful for the bounded Phase 26 world view. The UI remains downstream of derived read models.

## Explanation impact

Typed Explanation IR is decoded and rendered with explicit evidence state, confidence, comparison context, checkpoint, and trace count. Missing analysis is shown as unavailable, never synthesized into prose.

## Persistence impact

None. UI session state, timeline samples, locale, panel selection, stream queues, and rendered explanations are not persisted in authoritative snapshots. Existing runtime persistence remains separate from observer protobufs.

## Cross-domain effects

The UI exposes coupled physical → mana → resolution → action → population metrics so developers can inspect directionality without feeding analytical labels back into those domains.

## Risks

- Tauri v1 platform dependencies revealed a hidden shell build gap; migration to Tauri 2/WebKitGTK 4.1 resolved it and made the desktop crate part of workspace gates.
- A spatial heat map could be mistaken for exact geometry; the UI must label it as a bounded chunk projection.
- Client-side localized metadata could drift from Rust rendering; schema IDs and evidence fields remain canonical and visible.
- Rapid play cadence could create overlapping invokes; the session hook must serialize advances and use bounded history.

## Documentation changes

Update observer/UI architecture, views, protocol, performance diagnostics, roadmap Phase 26, domain coverage, changelog, and this plan.

## TODO changes

Add and complete a Phase 26 rich observer milestone covering the real protocol connection, bounded world projection, stream consumption, and Explanation IR UI. Preserve `TODO-PERF-001` until a general benchmark framework exists.

## Decision log

- 2026-07-13: Phase 26 uses a real causally bootstrapped runtime; disconnected browser mode contains no fabricated simulation data.
- 2026-07-13: React receives only protobuf-derived simulation data. Tauri command arguments control session execution, not authoritative content.
- 2026-07-13: The initial map is a bounded chart chunk projection; it does not claim global geography or per-resident rendering.
- 2026-07-13: Visual direction is a quiet, information-dense dark scientific tool, explicitly not a sci-fi control panel.
- 2026-07-13: The dormant Tauri 1 shell was migrated to Tauri 2 because the reference Linux environment provides the current WebKitGTK 4.1/libsoup3 stack.
- 2026-07-13: Release diagnostics measured negotiation + initial/four-tick stream at 0.01 s, world projection below 0.01 s display precision, and the 192-tick replay-verified Explanation analysis at 2.19 s; these are not scale claims.
- 2026-07-13: Final verification passed strict workspace clippy, every workspace test, protobuf validation, TypeScript checks, production Vite build, connected native Tauri rendering, and desktop/mobile browser checks without console errors or horizontal overflow.

## Progress

- [x] Architecture and boundary audit.
- [x] Observer spatial projection and codecs.
- [x] Tauri live session and tests.
- [x] React client and bounded session state.
- [x] Rich responsive UI.
- [x] Explanation and localization integration.
- [x] Verification, benchmark diagnostics, and documentation.
