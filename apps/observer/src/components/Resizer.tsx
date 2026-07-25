/**
 * Panel resizer.
 *
 * A hairline drag handle on the inner edge of a side panel. Pointer capture keeps the drag
 * alive across the whole window, and the keyboard adjusts in steps so the width is reachable
 * without a pointer.
 */

import { useRef, type PointerEvent as ReactPointerEvent } from "react";

const STEP = 24;

export function Resizer({
  edge,
  value,
  min,
  max,
  onChange,
  label,
}: {
  /** Which edge of the panel the handle sits on. */
  edge: "left" | "right";
  value: number;
  min: number;
  max: number;
  onChange(width: number): void;
  label: string;
}) {
  const origin = useRef<{ x: number; width: number } | undefined>(undefined);
  const clamp = (width: number) => Math.max(min, Math.min(max, Math.round(width)));

  const onPointerDown = (event: ReactPointerEvent<HTMLDivElement>) => {
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    origin.current = { x: event.clientX, width: value };
  };

  const onPointerMove = (event: ReactPointerEvent<HTMLDivElement>) => {
    const start = origin.current;
    if (start === undefined) return;
    const delta = event.clientX - start.x;
    onChange(clamp(start.width + (edge === "right" ? delta : -delta)));
  };

  const release = (event: ReactPointerEvent<HTMLDivElement>) => {
    origin.current = undefined;
    event.currentTarget.releasePointerCapture?.(event.pointerId);
  };

  return (
    <div
      className="resizer"
      data-edge={edge}
      role="separator"
      aria-orientation="vertical"
      aria-label={label}
      aria-valuenow={value}
      aria-valuemin={min}
      aria-valuemax={max}
      tabIndex={0}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={release}
      onPointerCancel={release}
      onDoubleClick={() => onChange(clamp((min + max) / 2))}
      onKeyDown={(event) => {
        const towards = edge === "right" ? 1 : -1;
        if (event.key === "ArrowLeft") {
          event.preventDefault();
          onChange(clamp(value - STEP * towards));
        } else if (event.key === "ArrowRight") {
          event.preventDefault();
          onChange(clamp(value + STEP * towards));
        }
      }}
    />
  );
}
