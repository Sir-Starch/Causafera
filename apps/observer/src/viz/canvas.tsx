/**
 * Canvas surface.
 *
 * Charts are drawn on a 2D canvas rather than as DOM nodes: the transition ledger alone can
 * carry sixty-four marks per frame, and the transect redraws on every advance. WebGPU is not
 * warranted at this data scale, and a DOM chart at this density costs more than it returns.
 *
 * The component owns device-pixel scaling and resize observation; callers only draw.
 */

import { useCallback, useEffect, useRef, useState, type PointerEvent, type ReactNode } from "react";

export interface Frame {
  width: number;
  height: number;
}

export type DrawFn = (context: CanvasRenderingContext2D, frame: Frame) => void;

interface CanvasSurfaceProps {
  height: number;
  draw: DrawFn;
  className?: string;
  label: string;
  onProbe?: (point: { x: number; y: number; frame: Frame } | undefined) => void;
  onActivate?: (point: { x: number; y: number; frame: Frame }) => void;
  children?: ReactNode;
}

export function CanvasSurface({
  height,
  draw,
  className,
  label,
  onProbe,
  onActivate,
  children,
}: CanvasSurfaceProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [frame, setFrame] = useState<Frame>({ width: 0, height });

  useEffect(() => {
    const host = hostRef.current;
    if (host === null) return undefined;
    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (entry === undefined) return;
      const width = Math.max(1, Math.round(entry.contentRect.width));
      setFrame((current) => (current.width === width ? current : { width, height }));
    });
    observer.observe(host);
    setFrame({ width: Math.max(1, Math.round(host.clientWidth)), height });
    return () => observer.disconnect();
  }, [height]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (canvas === null || frame.width === 0) return;
    const ratio = Math.min(window.devicePixelRatio || 1, 2);
    canvas.width = Math.round(frame.width * ratio);
    canvas.height = Math.round(frame.height * ratio);
    const context = canvas.getContext("2d");
    if (context === null) return;
    context.setTransform(ratio, 0, 0, ratio, 0, 0);
    context.clearRect(0, 0, frame.width, frame.height);
    draw(context, frame);
  }, [draw, frame]);

  const toPoint = useCallback(
    (event: PointerEvent<HTMLDivElement>) => {
      const host = hostRef.current;
      if (host === null) return undefined;
      const rect = host.getBoundingClientRect();
      return { x: event.clientX - rect.left, y: event.clientY - rect.top, frame };
    },
    [frame],
  );

  return (
    <div
      ref={hostRef}
      className={className === undefined ? "chart__surface" : `chart__surface ${className}`}
      style={{ height }}
      role="img"
      aria-label={label}
      onPointerMove={
        onProbe === undefined
          ? undefined
          : (event) => {
              onProbe(toPoint(event));
            }
      }
      onPointerLeave={onProbe === undefined ? undefined : () => onProbe(undefined)}
      onPointerDown={
        onActivate === undefined
          ? undefined
          : (event) => {
              const point = toPoint(event);
              if (point !== undefined) onActivate(point);
            }
      }
    >
      <canvas ref={canvasRef} />
      {children}
    </div>
  );
}

/** Resolve a CSS custom property to a concrete colour for canvas drawing. */
export function cssColor(name: string, fallback: string): string {
  if (typeof document === "undefined") return fallback;
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value.length > 0 ? value : fallback;
}

/** Read the whole palette once per draw rather than per mark. */
export function readPalette(): Record<string, string> {
  return {
    ink: cssColor("--ink", "#e4eaee"),
    inkDim: cssColor("--ink-dim", "#9baab6"),
    inkFaint: cssColor("--ink-faint", "#6b7a86"),
    inkGhost: cssColor("--ink-ghost", "#4a565f"),
    rule: cssColor("--rule", "rgba(167,190,205,0.15)"),
    ruleFaint: cssColor("--rule-faint", "rgba(167,190,205,0.09)"),
    ruleGhost: cssColor("--rule-ghost", "rgba(167,190,205,0.05)"),
    ruleStrong: cssColor("--rule-strong", "rgba(167,190,205,0.26)"),
    ruleBright: cssColor("--rule-bright", "rgba(196,216,228,0.42)"),
    mana: cssColor("--sig-mana", "#c28614"),
    trace: cssColor("--sig-trace", "#28a59f"),
    resolution: cssColor("--sig-resolution", "#936ece"),
    life: cssColor("--sig-life", "#4ba45c"),
    physical: cssColor("--sig-physical", "#4791ce"),
    refused: cssColor("--sig-refused", "#ce533e"),
    beacon: cssColor("--beacon", "#6fd8cf"),
    ramp100: cssColor("--ramp-100", "#edd3b0"),
    ramp300: cssColor("--ramp-300", "#d7a65a"),
    ramp500: cssColor("--ramp-500", "#9f6e13"),
    ramp700: cssColor("--ramp-700", "#5f4110"),
    sheet: cssColor("--sheet", "#0b1016"),
  };
}

export const MONO_FONT =
  '10px ui-monospace, "JetBrains Mono", "DejaVu Sans Mono", SFMono-Regular, Menlo, monospace';
export const MONO_FONT_SM =
  '9px ui-monospace, "JetBrains Mono", "DejaVu Sans Mono", SFMono-Regular, Menlo, monospace';
