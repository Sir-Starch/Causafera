# Phase 26 Observer Application

The desktop observer is a Tauri 2 + React application in `apps/observer`. It is a read-only client
of observer protocol v1, with a narrow exception for session execution controls that select the
seed, reset the session, or request a bounded number of scheduler ticks.

## Data Path

```text
Runtime / ExperimentRunner
    → ObserverSnapshot / ObserverWorldSnapshot / ExplanationReport
    → ProtocolHandler / ObserverStreamHub
    → protobuf v1 bytes through Tauri commands
    → TypeScript decoder
    → React view state
```

React never receives `Runtime`, `RuntimeState`, terrain storage, mana fields, actor state, or
provenance stores. It receives decoded protobuf values only. The disconnected browser build shows
an explicit unavailable state and never substitutes demonstration data.

## Session Envelope

The default interactive session uses a real deterministic runtime with eight promoted actors, two
sensors per actor, at most three active chunks in the demonstrated configuration, and a causal
bootstrap population of 512. The comparative Explanation query runs the existing replay-verified
192-tick control/intervention experiment within its 16-population in-memory experiment envelope.

Pause is local: the UI stops requesting tick batches. Tick batches are limited to 64 by the Rust
session. The live summary stream uses `latest-state-wins` with capacity one. Client timeline history
is a 96-entry FIFO.

## Presentation

The UI is a restrained dark scientific tool rather than a fictional control panel. Human labels,
locale, color, selected view, selected chunk, plots, and animations are non-authoritative. Supported
localization resources (`en`, `ru`, `zh-Hans`, `de`, and `es`) map opaque Explanation schema IDs to
presentation labels after IR decoding and provide interactive interface switching with browser persistence.

## Development

```text
pnpm install
pnpm --dir apps/observer desktop
```

Linux uses the installed WebKitGTK 4.1 stack through Tauri 2. When both Wayland and X11 are
available, the `desktop` launcher automatically selects XWayland with software rendering. This
avoids WebKitGTK protocol failures observed with some NVIDIA and remote-desktop configurations.
It changes only presentation and cannot affect simulation state.

Native Wayland remains available for platform testing:

```text
pnpm --dir apps/observer desktop:raw
# or
CAUSAFERA_NATIVE_WAYLAND=1 pnpm --dir apps/observer desktop
```
