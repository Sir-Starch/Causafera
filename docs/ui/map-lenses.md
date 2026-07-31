# Map Lenses

The implementation contract of the chart instrument in `apps/observer/src/map`. It exists so a
later contributor can connect a new domain to the map without touching the renderer, and so the
boundary between what the observer measures and what it constructs stays explicit.

Companion documents:

- `docs/ui/map-perspectives.md` — which knowledge states a map may show
- `docs/ui/observer-projection-gaps.md` — the read models the lenses are waiting on
- `docs/ui/observer-application.md` — the surrounding frontend architecture

## What a lens is

A lens is one class of information projected onto the chart. The renderer draws six kinds of
geometry and knows nothing about what any of them mean:

| Layer | Geometry | Drawn at | Mounted as |
|-------|----------|----------|------------|
| `surface` | a continuous field over the whole surveyed extent, painted as one image | every scale | primary, and overlay composited over it |
| `field` | a scalar per chunk, painted as a tint of the lens hue | every scale | primary only |
| `symbols` | a mark at a chunk centre — a proportional circle, or a fixed glyph | chunk and cell scale | primary or overlay |
| `cells` | a mark at a real cell position inside a chunk | cell scale only | primary or overlay |
| `vectors` | a directed magnitude between two chunk centres | every scale | primary or overlay |
| `isolines` | polylines in chart space, clipped to the charted extent | every scale | primary or overlay |

The last column is contract, not trivia: a lens that returns a layer the renderer does not consume in
the position it is mounted draws nothing, silently and with no error anywhere. `field` is the one
that is primary-only, because a tint per chunk is the base fill of the sheet and two of them
stacked would be a wash over a wash rather than two readings.

An overlay `surface` composites onto whatever is already painted, so a lens that draws one has to
mean it: its ramp must be transparent where the quantity is absent, or it will simply hide the
primary field. Surface water is the case the rule exists for — it is nothing without ground beneath
it, and its alpha starts at zero.

`surface` and `field` answer the same question at different resolutions, and a lens supplies
whichever it can: a `surface` when the observer has sent the lattice, a `field` when one value per
chunk is all there is. One value over one area is honestly drawn as one tint, so the fallback is not
a degraded surface — it is the correct drawing of what arrived.

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

## Continuous fields

`src/map/field.ts` assembles received lattices into one `ChartField` over the surveyed extent, and
`src/map/surface.ts` paints it. Everything the map says about a field is a reading of that one
surface — the tint, the relief shading, the contours and the hover readout — so a gradient crossing a
chunk boundary is drawn as one gradient and any discontinuity in the drawing is a discontinuity in
the world.

Four rules govern it:

1. **Only received samples enter.** Ground the observer has not been given is marked uncovered and
   drawn as unsurveyed. The field is never interpolated across a hole.
2. **Interpolation is stated.** Between samples the field is resampled with a Catmull-Rom kernel,
   which passes exactly through every measurement and is clamped to the bracketing pair so it cannot
   reach a value outside the local range. Lenses that draw it say so in their caveat.
3. **Contours follow the same interpolant.** A coarse lattice is refined before marching squares
   runs, so the line and the tint underneath it are readings of one field rather than two.
4. **Shading is presentation.** The Lambertian term is computed here and never returns to the
   runtime (INV-022). It is applied only to fields whose gradient is a slope: mana intensity is not a
   height, so it is not hillshaded.

The painted image is cached against the `signature` a lens supplies, which must identify the
measurements exactly. Panning and zooming then cost a scaled blit, and a surface is repainted only
when the observer sends new values.

## Availability that follows what arrived

A lens may supply `availabilityFor(context)` instead of relying on its constant `availability`. The
mana lenses use it: they report `preview` while the received lattice edge is too coarse to draw
without upsampling and `observed` when it is not. Refining `chunk_extent` therefore promotes them
with no change to the catalogue, and coarsening it cannot quietly overstate what is drawn.

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

To connect a lens to a per-cell lattice instead, build a `ChartField` through
`src/map/rasterFields.ts` and return it as a `surface` with a `signature`, a `style` and a `format`.
A volumetric lattice must state which reduction to plan view it is showing, because the runtime
projects the volume unreduced precisely so that choice stays a reading of the field.

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

Culling is by viewport (`visibleChunks`), so a screenful costs the same whether the chart holds nine
chunks or thousands. The demonstration configuration is not an assumption anywhere in the renderer.

A surface is resolved in sample space rather than screen space, so painting it costs what has been
surveyed rather than how far the chart is zoomed in; the canvas scales the cached image.

## The graticule, and why it no longer rules the sheet

A lattice ruled across the paper turns any map into a grid of squares. Over a drawn field the chunk
lattice is therefore stated by ticks at the intersections and by the coordinate labels, and only
resolves into rules once a chunk is large enough on screen that its boundary is a reading rather than
a frame. A lens with only chunk aggregates keeps the full rules, because there the boundary really is
where one measurement stops and the next begins.

## Water

The `hydrology` group holds three lenses: the same quantity in three places — ponded surface water,
water held in the unsaturated zone, and water in the saturated zone.

Three things about them are contract rather than styling:

1. **Volumes, never depths.** A water volume is an exact count of cubic millimetres. A depth is that
   volume over a cell area, the `HydrologyGridMetric` carrying that area is declared per chart, and
   it is not projected to the observer — so the chart shows cubic metres and says so. Rendering a
   millimetre figure here would be a number invented on this side of the wire.
2. **The unsigned band is a separate band.** A hydrology lattice carries its values in
   `unsignedValues` (`BigUint64Array`) and leaves the signed `values` empty, because the upper half
   of a `u64` has no image in a double. `unsignedSurfaceField` converts to doubles to paint, and
   `unsignedPeak` reads the exact maximum for the signature and the legend, so a repaint is decided
   by a count rather than by a rounded one.
3. **No water bodies.** There is no lake, river, wetland or catchment lens, and there will not be
   one drawn from this data. Those are readings a viewer may take; they are not simulation state, and
   nothing an observer computes may travel back (INV-022, and the source audit in
   `tools/audit/test-hydrology-production-boundaries.mjs`).

There is deliberately no fourth lens over the per-cell delta window. That window exists and the
frontend reads it, but it is capped at 64 entries taken in canonical address order, so on a chart of
nine thousand cells the entries it carries are the lowest-addressed changed cells rather than a
sample of where water moved. Drawn as marks they would cluster in one corner and read as a claim
about geography that the selection rule, not the world, produced. The window is presented as a table
in Flux instead, where canonical ordering is visible as ordering. A map overlay needs either a
spatially representative window or a much larger cap, which is `TODO-OBS-006`.

Surface water is painted with alpha rising from zero, so a dry cell shows the ground beneath rather
than the floor of a blue ramp. That is what lets it sit over the relief as an overlay and read as a
shape against land. Soil and groundwater use a separate, lower-chroma ramp on purpose: three lenses
over the same blue would look like one lens with a bug.

## What the chart opens on, and why

The default primary lens is measured relief, with surface water and contours over it.

Water needs ground under it to mean anything. The surface-water lens paints only where water stands,
so over the hypsometric relief it reads the way water reads on a chart — a shape against land, in the
low ground the solver routed it into — and the contours state the elevation it is answering to.

Contours were previously kept out of the default set for a generation reason that has since closed:
`terrain_cells` used to derive elevation from chunk-local coordinates only, so every chunk repeated
the same diagonal ridge and the chart carried a thirty-metre scarp on every chunk boundary. That
closed with `TODO-GEO-005`. The relief lens's caveat about the step being world state rather than a
seam remains true; it is just no longer describing a defect.

The mana field opened the chart while it was the only field the runtime maintained continuously
across the whole charted extent. That is no longer the case, and it keeps every lens it had one
click away.

## What the map deliberately does not do

- It does not join charts. One chart at one containment layer is projected at a time, because
  chunk coordinates are chart-qualified lattice addresses and no seamless global surface exists
  (INV-036).
- It does not draw terrain it was not given. Ground outside the received extent is hatched as
  unsurveyed, which is a finding rather than a blank.
- It does not imply agent knowledge. Everything drawn is the objective observer projection; the
  subjective perspectives in `docs/ui/map-perspectives.md` are `awaiting` lenses.
