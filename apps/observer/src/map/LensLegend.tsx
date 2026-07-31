/**
 * The legend cartouche.
 *
 * Sits on the chart itself, the way a legend belongs to a map rather than to a page. It
 * carries the base ramp with its real extremes, a key for every overlay mark, the current
 * lens's availability, and the caveat that availability implies.
 */

import { useState } from "react";

import { SIGNAL_VARIABLE } from "../observer/models";
import type { ObserverLocale } from "../observer/format";
import {
  AVAILABILITY_TITLE,
  availabilityOf,
  type Lens,
  type LensContext,
  type LensLayers,
} from "./lens";
import { rampSwatches } from "./surface";

/** The stepped ramp mirrors the alpha the renderer applies to a field value. */
const RAMP_STEPS = [0.07, 0.18, 0.29, 0.4, 0.5, 0.57];

interface LensLegendProps {
  locale: ObserverLocale;
  context: LensContext;
  primary?: Lens;
  primaryLayers: LensLayers;
  overlays: { lens: Lens; layers: LensLayers }[];
  labels: { legend: string; noField: string; overlays: string };
}

export function LensLegend({
  locale,
  context,
  primary,
  primaryLayers,
  overlays,
  labels,
}: LensLegendProps) {
  const field = primaryLayers.field;
  const surface = primaryLayers.surface;
  const [folded, setFolded] = useState(false);
  const state = primary === undefined ? undefined : availabilityOf(primary, context);

  return (
    <div className="chart-map__legend" data-folded={folded}>
      <div className="chart-map__legend-head">
        <span className="eyebrow">{labels.legend}</span>
        {state !== undefined && !folded && (
          <span className="lens-availability" data-availability={state}>
            {AVAILABILITY_TITLE[state][locale]}
          </span>
        )}
        <button
          type="button"
          className="chart-map__legend-fold"
          aria-expanded={!folded}
          onClick={() => setFolded((current) => !current)}
        >
          {folded ? "▲" : "▼"}
        </button>
      </div>

      {primary !== undefined && (
        <>
          <strong className="chart-map__legend-title">{primary.title[locale]}</strong>
          {surface !== undefined ? (
            /* The ramp shows the colours the surface is actually painted in,
               between the extremes of the measurements it was built from. */
            <div className="chart-map__ramp">
              <span className="chart-map__ramp-bar">
                {rampSwatches(surface.style.ramp, 24).map((colour, index) => (
                  <span
                    key={colour + index}
                    className="chart-map__ramp-step"
                    style={{ background: colour }}
                  />
                ))}
              </span>
              <span className="chart-map__ramp-scale numeric">
                <span>{surface.format(surface.field.min)}</span>
                <span>{surface.format(surface.field.max)}</span>
              </span>
            </div>
          ) : field === undefined ? (
            <p className="chart-map__legend-note">{labels.noField}</p>
          ) : (
            <div
              className="chart-map__ramp"
              style={{ ["--signal" as string]: SIGNAL_VARIABLE[primary.signal] }}
            >
              <span className="chart-map__ramp-bar">
                {RAMP_STEPS.map((opacity) => (
                  <span key={opacity} className="chart-map__ramp-step" style={{ opacity }} />
                ))}
              </span>
              <span className="chart-map__ramp-scale numeric">
                <span>{field.format(field.min)}</span>
                <span>{field.format(field.max)}</span>
              </span>
            </div>
          )}
          {primary.caveat !== undefined && (
            <p className="chart-map__legend-note">{primary.caveat[locale]}</p>
          )}
        </>
      )}

      {overlays.length > 0 && (
        <>
          <span className="eyebrow chart-map__legend-divider">{labels.overlays}</span>
          <ul className="chart-map__keys">
            {overlays.map(({ lens, layers }) => (
              <li
                key={lens.id}
                style={{ ["--signal" as string]: SIGNAL_VARIABLE[lens.signal] }}
                data-availability={availabilityOf(lens, context)}
              >
                <span className={`chart-map__key-mark chart-map__key-mark--${keyShape(layers)}`} />
                {lens.title[locale]}
              </li>
            ))}
          </ul>
        </>
      )}
    </div>
  );
}

/** The key mark mirrors whichever geometry the overlay actually contributes. */
function keyShape(layers: LensLayers): string {
  // Before marks, because an overlay that paints a field paints the largest
  // thing on the sheet, and a key that showed its isolines instead would
  // describe the smaller half of what it drew.
  if (layers.surface !== undefined) return "wash";
  if (layers.isolines !== undefined && layers.isolines.length > 0) return "line";
  if (layers.vectors !== undefined && layers.vectors.length > 0) return "arrow";
  if (layers.cells !== undefined && layers.cells.length > 0) return "diamond";
  if (layers.symbols !== undefined && layers.symbols.length > 0) return "circle";
  return "empty";
}
