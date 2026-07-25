# Observer Application

The desktop observer is a Tauri 2 + React application in `apps/observer`. It is a read-only client
of observer protocol v1, with a narrow exception for session execution controls that select the
seed, reset the session, or request a bounded number of scheduler ticks.

## Data Path

```text
Runtime / ExperimentRunner
    → ObserverSnapshot / ObserverWorldSnapshot / ExplanationReport
    → ProtocolHandler / ObserverStreamHub
    → protobuf v1 bytes through Tauri commands
    → TypeScript decoder (@causafera/observer-protocol)
    → presentation models (src/observer/models.ts)
    → React areas
```

React never receives `Runtime`, `RuntimeState`, terrain storage, mana fields, actor state, or
provenance stores. It receives decoded protobuf values only. The disconnected browser build shows
an explicit unavailable state and never substitutes demonstration data.

## Frontend Architecture

| Layer | Location | Responsibility |
|-------|----------|----------------|
| Transport | `src/observer/transport.ts` | Byte channel, digest verification, exchange log |
| Session | `src/observer/session.ts` | Connection lifecycle, run loop, bounded buffers |
| Store | `src/observer/store.ts` | `useSyncExternalStore` container, feed demand registry |
| Presentation models | `src/observer/models.ts` | Typed reductions of protocol payloads |
| Capability register | `src/observer/capability.ts` | What the instrument can and cannot observe |
| Claim descriptors | `src/observer/claims.ts` | How to read each Explanation claim schema |
| Design system | `src/design/*.css` | Tokens, chrome, surfaces, controls, data, charts |
| Visualisation | `src/viz/*` | Canvas chart recorder, chart profile, condition ladder |
| Areas | `src/areas/*` | Observatory, Survey, Flux, Assay, Instrument |

There is no state-management dependency. The store is about a hundred lines and is driven from
outside React by the session controller, so the connection is not tied to component mounting.

### Scoped feeds

An area declares the feeds it needs with `useFeed`. The session polls the world query only while at
least one mounted consumer demands it, which is the frontend half of the scoped-subscription rule
in `docs/observer/backpressure.md`: a closed panel produces no traffic.

### Bounded buffers

| Buffer | Bound | Location |
|--------|-------|----------|
| Ticks per advance | 64 | `src-tauri/src/session.rs` |
| Material surface transition window | 64 | `causafera-observer-api` |
| Observer-side summary series | 256 frames | `HISTORY_CAPACITY` |
| Exchange log | 120 entries | `EXCHANGE_CAPACITY` |
| Runtime stream | capacity 1, latest-state-wins | `src-tauri/src/session.rs` |

The summary series is presentation state assembled from received frames. It is not authoritative
history, and every surface that plots it says so.

## Areas

| Area | Reads | Purpose |
|------|-------|---------|
| Observatory | Runtime summary, world chunks, transition window | Run identity, instrument cluster, mana field, causal accretion, action admission |
| Survey | World chunks, transition window | Chart profile of active chunks, chunk register, chunk inspector |
| Flux | Runtime summary series, transition window, gate window | Rate recorders, surface condition ladder, transition ledger, gate transitions |
| Assay | Explanation IR | Typed claims with evidence state, confidence, comparison, trace anchors |
| Instrument | Negotiation, exchange log, capability register | Protocol state, real transport measurements, coverage register |

Areas are independent. Switching areas changes no simulation state and no other area.

## Session Envelope

The default interactive session uses a real deterministic runtime with eight promoted actors, two
sensors per actor, at most three active chunks in the demonstrated configuration, and a causal
bootstrap population of 512. The comparative Explanation query runs the existing replay-verified
control/intervention experiment within its in-memory experiment envelope.

Pause is local: the UI stops requesting tick batches.

## Presentation

The visual system is a black outline atlas: white ink on black chart paper, hairline rules, square
corners, and hatching for unsurveyed ground. Nothing glows, nothing is glass, nothing carries a
gradient except the paper itself.

Beneath the whole application lies the chart sheet (`src/components/TerraIncognita.tsx`): an SVG
outline map with coastlines, engraved water lining, interior contour rings, a graticule, portolan
rhumb lines from two compass nodes, depth soundings, a compass rose, and coastlines that break into
survey dashes where the survey was never closed. It is generated from fixed sums of sinusoids, so it
is identical in every session and cannot be mistaken for data (INV-022). Plates sit on it with a
flat, partly transparent fill, so the sheet reads through without harming text.

Colour is disciplined: **the interface chrome is monochrome ink, and hue is reserved entirely for
measured quantities**. A coloured mark on screen therefore always means a simulation quantity and
never decoration. Six signal hues carry mana, causal traces, resolution, population, physical events
and refusals. Evidence states use a separate reserved status palette and always pair a hue with a
word; `Unknown` is drawn as hatched unsurveyed ground rather than as an error.

Selection and activity are drawn, not lit: the run control inverts to paper-on-ink while advancing,
a followed trace anchor is inked solid, a selected register row is ruled, and focus is a hairline.

The signal palette was generated in OKLCH and validated as a categorical palette against the chart
surface for lightness band, chroma floor, protanopia and deuteranopia separation, normal-vision
separation, and contrast. Do not hand-edit the hues in `src/design/tokens.css`; regenerate and
revalidate them.

Human labels, locale, colour, selected area, selected chunk, plots, and animations are
non-authoritative. Russian and English metadata map opaque Explanation schema IDs to presentation
labels after IR decoding.

### Charts

Charts are drawn on a 2D canvas with device-pixel scaling. WebGPU is not used: the current data
scale does not warrant it, and the documented visualisation direction favours 2D and restrained
2.5D. Every recorder carries a single value axis; two magnitudes get two recorders rather than two
scales.

Chart surfaces follow the same line-work discipline: dotted grids, hairline strokes, and **engraved
hatching** instead of gradient washes beneath a curve or across a relief band. Hatching keeps the
plate reading as drawn line-work and survives printing and forced colours, which a translucent fill
does not.

The chart profile draws active chunks as a stacked register — relief, mana, population, causal
activity — in chart coordinate order. It is a profile, not a map: adjacency is ordering, not
measured distance.

## Keyboard

| Key | Action |
|-----|--------|
| `1`–`5` | Go to area |
| `Space` | Run / hold |
| `→` | Advance one batch |
| `I` | Toggle the inspector dock |
| `Ctrl`/`Cmd` + `K` | Command palette |

## Responsive Behaviour

The shell reorganises rather than shrinking. Below 86rem the wordmark yields, below 76rem the
digest plates yield, below 68rem the rail collapses to marks and the inspector becomes an overlay,
and below 60rem the brand and locale control yield. The transport is the last control to lose
space.

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

### Capture and replay

Frontend work often happens without a graphical session. The capture example drives the same
`ObserverSession` the desktop shell uses and writes the protocol frames it produced:

```text
pnpm --dir apps/observer capture     # writes apps/observer/dev/replay/capture.json
pnpm --dir apps/observer smoke       # renders every area against the capture
pnpm --dir apps/observer dev         # browser session replaying the capture
```

The capture holds authentic deterministic runtime output, not fixture data, and it is decoded by
the production codec. It is available in development builds only, the branch that loads it is
compiled out of production bundles, and the interface marks a replaying session in the meridian
bar, the marginalia strip, and the instrument area. When the capture is absent the observer
reports itself unattached, as it must (INV-039).

`pnpm --dir apps/observer smoke` server-renders every area and inspector dock, in both locales,
with and without a selection, against real captured payloads. It exits successfully when no
capture is present.

## Related Documents

- `docs/ui/views.md` — delivered and planned views
- `docs/ui/observer-projection-gaps.md` — projections the frontend is waiting on
- `docs/observer/architecture.md` — observer layer providing view data
- `docs/observer/backpressure.md` — delivery policies and scoped subscriptions
