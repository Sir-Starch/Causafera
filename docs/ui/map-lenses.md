# Map Lenses

The implementation contract of the chart instrument in `apps/observer/src/map`. It exists so a
later contributor can connect a new domain to the map without touching the renderer, and so the
boundary between what the observer measures and what it constructs stays explicit.

Companion documents:

- `docs/ui/map-perspectives.md` — which knowledge states a map may show
- `docs/ui/observer-projection-gaps.md` — the read models the lenses are waiting on
- `docs/ui/observer-application.md` — the surrounding frontend architecture

## What a lens is

A lens is one class of information projected onto the chart. The renderer draws five kinds of
geometry and knows nothing about what any of them mean:

| Layer | Geometry | Drawn at |
|-------|----------|----------|
| `field` | a scalar per chunk, painted as a tint of the lens hue | every scale |
| `symbols` | a mark at a chunk centre — a proportional circle, or a fixed glyph | chunk and cell scale |
| `cells` | a mark at a real cell position inside a chunk | cell scale only |
| `vectors` | a directed magnitude between two chunk centres | every scale |
| `isolines` | polylines in chart space, clipped to the charted extent | every scale |

A lens returns whichever of those it can produce from the observer payloads it is handed. Adding a
domain means adding an entry to `src/map/lenses.ts`; it never means editing `src/map/ChartMap.tsx`.

## Availability is part of the contract

Every lens declares one of four states, and the interface draws the difference:

| State | Meaning | Treatment |
|-------|---------|-----------|
| `observed` | real observer data through the current protocol | solid glyph frame |
| `partial` | real data, but a narrow slice of what the lens names | half-filled corner mark |
| `preview` | an observer-side construction over real values, not a measurement | dotted corner mark |
| `awaiting` | no read model yet | listed in the catalogue only; selecting it draws the chart as unsurveyed and states what is missing |

`caveat` on a lens names the exact limitation or construction. Those sentences are the user-facing
half of `docs/ui/observer-projection-gaps.md` and should be kept in step with it.

## Preview projections

`src/map/preview.ts` holds everything the observer constructs rather than receives: inverse-distance
interpolation with marching-squares isolines, and neighbour differences drawn as vectors. It is a
separate module on purpose.

Two rules govern it:

1. **Arithmetic over received values only.** Nothing in that module may invent world content —
   no settlements, residents, routes or histories. It reduces chunk summaries the runtime sent.
2. **Anything built on it is `preview`.** The lens carries the mark, the legend carries the caveat,
   and interpolated geometry is clipped to the charted extent so a construction never paints
   knowledge over ground the observer never received.

## Connecting a real read model

To promote an `awaiting` lens once its projection lands:

1. Extend `LensContext` in `src/map/lens.ts` with the decoded payload.
2. Supply it in `useChartContext` in `src/areas/ChartArea.tsx`.
3. Replace the entry's `layers` with a real projection and change `availability`.
4. Set `cellProjection` honestly — `none` makes the renderer hatch the cell lattice to say the
   value is a chunk aggregate rather than a cell measurement.
5. Draw a glyph in `src/map/LensIcon.tsx`; without one the lens falls back to a survey mark and
   still works.

Nothing else in the map changes. The dock, the legend, the catalogue, the hover readout and the
level-of-detail rules all read the lens contract.

## Scale and level of detail

World units are chart units; one chunk is `CHUNK_UNITS` square. Detail follows legibility, not
preference:

| Chunk size on screen | Detail | What appears |
|---------------------|--------|--------------|
| under 30 px | `field` | the field tint only |
| 30–190 px | `chunk` | borders, coordinates, the field value, symbols |
| over 190 px | `cell` | the 32³ cell lattice and cell marks at real positions |

Culling is by viewport (`visibleChunks`), so a screenful costs the same whether the chart holds
three chunks or thousands. The three-chunk demonstration configuration is not an assumption
anywhere in the renderer.

## What the map deliberately does not do

- It does not join charts. One chart at one containment layer is projected at a time, because
  chunk coordinates are chart-qualified lattice addresses and no seamless global surface exists
  (INV-036).
- It does not draw terrain it was not given. Ground outside the received extent is hatched as
  unsurveyed, which is a finding rather than a blank.
- It does not imply agent knowledge. Everything drawn is the objective observer projection; the
  subjective perspectives in `docs/ui/map-perspectives.md` are `awaiting` lenses.
