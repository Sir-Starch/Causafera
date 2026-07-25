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
  type Lens,
  type LensLayers,
} from "./lens";

/** The stepped ramp mirrors the alpha the renderer applies to a field value. */
const RAMP_STEPS = [0.07, 0.18, 0.29, 0.4, 0.5, 0.57];

interface LensLegendProps {
  locale: ObserverLocale;
  primary?: Lens;
  primaryLayers: LensLayers;
  overlays: { lens: Lens; layers: LensLayers }[];
  labels: { legend: string; noField: string; overlays: string };
}

export function LensLegend({
  locale,
  primary,
  primaryLayers,
  overlays,
  labels,
}: LensLegendProps) {
  const field = primaryLayers.field;
  const [folded, setFolded] = useState(false);

  return (
    <div className="chart-map__legend" data-folded={folded}>
      <div className="chart-map__legend-head">
        <span className="eyebrow">{labels.legend}</span>
        {primary !== undefined && !folded && (
          <span className="lens-availability" data-availability={primary.availability}>
            {AVAILABILITY_TITLE[primary.availability][locale]}
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
          {field === undefined ? (
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
                data-availability={lens.availability}
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
  if (layers.isolines !== undefined && layers.isolines.length > 0) return "line";
  if (layers.vectors !== undefined && layers.vectors.length > 0) return "arrow";
  if (layers.cells !== undefined && layers.cells.length > 0) return "diamond";
  if (layers.symbols !== undefined && layers.symbols.length > 0) return "circle";
  return "empty";
}
