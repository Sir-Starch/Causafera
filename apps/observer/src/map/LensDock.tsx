/**
 * The lens dock.
 *
 * A floating instrument bar over the chart: one glyph per lens, grouped by domain and divided
 * by hairlines, with the name and its availability arriving on hover. It costs the layout no
 * height, which is the point — the chart is the surface, and its controls sit on it the way a
 * cartouche does.
 *
 * The base field is a single choice; overlays are toggles. Availability is drawn into the
 * glyph frame, so the difference between a measurement, a narrow slice and an observer-side
 * construction is visible before anything is selected.
 */

import { useState } from "react";

import { HoverCard, anchorOf, type HoverAnchor } from "../components/HoverCard";

import { SIGNAL_VARIABLE } from "../observer/models";
import type { ObserverLocale } from "../observer/format";
import {
  AVAILABILITY_TITLE,
  LENS_GROUPS,
  availabilityOf,
  canBePrimary,
  type Lens,
  type LensContext,
  type LensId,
} from "./lens";
import { LENSES } from "./lenses";
import { LensIcon } from "./LensIcon";

interface LensDockProps {
  locale: ObserverLocale;
  /** Availability of a lattice-backed lens depends on what arrived, not on the lens. */
  context: LensContext;
  primaryId: LensId;
  overlayIds: readonly LensId[];
  onPrimary(id: LensId): void;
  onToggleOverlay(id: LensId): void;
  onOpenCatalogue(): void;
  labels: { primary: string; overlays: string; catalogue: string; awaiting: string };
}

interface Hint {
  lens?: Lens;
  text?: string;
  anchor: HoverAnchor;
  role: string;
}

export function LensDock({
  locale,
  context,
  primaryId,
  overlayIds,
  onPrimary,
  onToggleOverlay,
  onOpenCatalogue,
  labels,
}: LensDockProps) {
  const [hint, setHint] = useState<Hint>();
  const stateOf = (lens: Lens) => availabilityOf(lens, context);
  const overlays = LENSES.filter(
    (lens) => lens.roles.includes("overlay") && lens.availability !== "awaiting",
  );
  const awaitingCount = LENSES.filter((lens) => lens.availability === "awaiting").length;

  const show = (event: { currentTarget: HTMLElement }, role: string, lens?: Lens, text?: string) => {
    setHint({ lens, text, anchor: anchorOf(event.currentTarget), role });
  };

  return (
    <div className="lens-dock" onPointerLeave={() => setHint(undefined)}>
      <div className="lens-dock__bar">
        <span className="lens-dock__label">{labels.primary}</span>
        {LENS_GROUPS.map((group) => {
          const lenses = LENSES.filter(
            (lens) =>
              lens.group === group.id && canBePrimary(lens) && lens.availability !== "awaiting",
          );
          if (lenses.length === 0) return null;
          return (
            <div key={group.id} className="lens-dock__group">
              {lenses.map((lens) => (
                <button
                  key={lens.id}
                  type="button"
                  className="lens-dock__key"
                  data-availability={stateOf(lens)}
                  data-role="primary"
                  aria-pressed={lens.id === primaryId}
                  aria-label={lens.title[locale]}
                  style={{ ["--signal" as string]: SIGNAL_VARIABLE[lens.signal] }}
                  onPointerEnter={(event) => show(event, labels.primary, lens)}
                  onFocus={(event) => show(event, labels.primary, lens)}
                  onClick={() => onPrimary(lens.id)}
                >
                  <LensIcon lensId={lens.id} />
                </button>
              ))}
            </div>
          );
        })}

        <span className="lens-dock__divider" />

        <span className="lens-dock__label">{labels.overlays}</span>
        <div className="lens-dock__group">
          {overlays.map((lens) => (
            <button
              key={lens.id}
              type="button"
              className="lens-dock__key"
              data-availability={stateOf(lens)}
              data-role="overlay"
              aria-pressed={overlayIds.includes(lens.id)}
              aria-label={lens.title[locale]}
              style={{ ["--signal" as string]: SIGNAL_VARIABLE[lens.signal] }}
              onPointerEnter={(event) => show(event, labels.overlays, lens)}
              onFocus={(event) => show(event, labels.overlays, lens)}
              onClick={() => onToggleOverlay(lens.id)}
            >
              <LensIcon lensId={lens.id} />
            </button>
          ))}
        </div>

        <span className="lens-dock__divider" />

        <button
          type="button"
          className="lens-dock__key lens-dock__key--catalogue"
          aria-label={labels.catalogue}
          onPointerEnter={(event) =>
            show(event, labels.catalogue, undefined, labels.awaiting.replace("{n}", String(awaitingCount)))
          }
          onFocus={(event) =>
            show(event, labels.catalogue, undefined, labels.awaiting.replace("{n}", String(awaitingCount)))
          }
          onClick={onOpenCatalogue}
        >
          <span className="lens-dock__catalogue-mark" aria-hidden="true">
            ⋯
          </span>
        </button>
      </div>

      {hint !== undefined && (
        <HoverCard anchor={hint.anchor}>
          <span className="hover-card__role">{hint.role}</span>
          {hint.lens === undefined ? (
            <strong>{hint.text}</strong>
          ) : (
            <>
              <strong>{hint.lens.title[locale]}</strong>
              <span className="lens-availability" data-availability={stateOf(hint.lens)}>
                {AVAILABILITY_TITLE[stateOf(hint.lens)][locale]}
              </span>
              <p>{hint.lens.detail[locale]}</p>
              {hint.lens.caveat !== undefined && (
                <p className="hover-card__caveat">{hint.lens.caveat[locale]}</p>
              )}
            </>
          )}
        </HoverCard>
      )}
    </div>
  );
}
