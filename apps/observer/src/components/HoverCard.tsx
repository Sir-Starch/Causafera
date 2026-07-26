/**
 * Hover card.
 *
 * Rendered into a portal at the document root and positioned in viewport coordinates, so it is
 * never clipped by the panel that raised it. Anything that overflows — a map frame, a scrolling
 * table, an inspector — would otherwise cut the explanation in half.
 *
 * The card flips above its anchor when it would leave the bottom of the window and is clamped
 * to the horizontal edges, so it stays whole wherever it is triggered.
 */

import { useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";

export interface HoverAnchor {
  /** Viewport rectangle of the element the card belongs to. */
  x: number;
  y: number;
  width: number;
  height: number;
}

const MARGIN = 8;
const GAP = 6;

export function HoverCard({ anchor, children }: { anchor: HoverAnchor; children: ReactNode }) {
  const cardRef = useRef<HTMLDivElement>(null);
  const [placement, setPlacement] = useState<{ left: number; top: number } | undefined>();

  useLayoutEffect(() => {
    const card = cardRef.current;
    if (card === null) return;
    const { width, height } = card.getBoundingClientRect();
    const left = Math.max(
      MARGIN,
      Math.min(anchor.x + anchor.width / 2 - width / 2, window.innerWidth - width - MARGIN),
    );
    const below = anchor.y + anchor.height + GAP;
    const top =
      below + height + MARGIN > window.innerHeight ? Math.max(MARGIN, anchor.y - height - GAP) : below;
    setPlacement({ left, top });
  }, [anchor]);

  if (typeof document === "undefined") return null;

  return createPortal(
    <div
      ref={cardRef}
      className="hover-card"
      role="tooltip"
      style={{
        left: placement?.left ?? anchor.x,
        top: placement?.top ?? anchor.y + anchor.height + GAP,
        visibility: placement === undefined ? "hidden" : "visible",
      }}
    >
      {children}
    </div>,
    document.body,
  );
}

/** Capture an element's viewport rectangle for the card to anchor against. */
export function anchorOf(element: HTMLElement): HoverAnchor {
  const rect = element.getBoundingClientRect();
  return { x: rect.left, y: rect.top, width: rect.width, height: rect.height };
}
