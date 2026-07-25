/**
 * Chart recorder.
 *
 * A single-axis line recorder over run ticks. Series handed to one recorder must share a
 * unit — two magnitudes never get two y-scales; they get two recorders.
 *
 * Interaction: a crosshair probe reads every series at the nearest received frame. The
 * observer only holds the frames it asked for, so the probe snaps to real samples rather
 * than interpolating a value that was never observed.
 */

import { useCallback, useMemo, useState } from "react";

import { SIGNAL_VARIABLE, type Series } from "../observer/models";
import { CanvasSurface, MONO_FONT, hatchPattern, readPalette, type Frame } from "./canvas";
import { axisLabel, linearScale, niceTicks } from "./scale";

const PADDING = { top: 12, right: 14, bottom: 20, left: 46 };

interface ChartRecorderProps {
  series: Series[];
  height?: number;
  label: string;
  valueFormat?: (value: number) => string;
  tickLabel: string;
  fillFirst?: boolean;
  legendLabels: Record<string, string>;
  emptyLabel: string;
}

interface Probe {
  index: number;
  x: number;
  ticks: number;
  values: { id: string; signal: Series["signal"]; value: number }[];
  side: "left" | "right";
  top: number;
}

export function ChartRecorder({
  series,
  height = 168,
  label,
  valueFormat = axisLabel,
  tickLabel,
  fillFirst = false,
  legendLabels,
  emptyLabel,
}: ChartRecorderProps) {
  const [muted, setMuted] = useState<ReadonlySet<string>>(() => new Set<string>());
  const [probe, setProbe] = useState<Probe>();

  const visible = useMemo(
    () => series.filter((entry) => !muted.has(entry.id) && entry.points.length > 0),
    [series, muted],
  );

  const domain = useMemo(() => {
    let tickMin = Number.POSITIVE_INFINITY;
    let tickMax = Number.NEGATIVE_INFINITY;
    let valueMax = Number.NEGATIVE_INFINITY;
    let valueMin = Number.POSITIVE_INFINITY;
    for (const entry of visible) {
      for (const point of entry.points) {
        tickMin = Math.min(tickMin, point.ticks);
        tickMax = Math.max(tickMax, point.ticks);
        valueMax = Math.max(valueMax, point.value);
        valueMin = Math.min(valueMin, point.value);
      }
    }
    if (!Number.isFinite(tickMin)) return undefined;
    return {
      tickMin,
      tickMax: tickMax === tickMin ? tickMin + 1 : tickMax,
      valueMin: Math.min(0, valueMin),
      valueMax: valueMax <= 0 ? 1 : valueMax * 1.08,
    };
  }, [visible]);

  const draw = useCallback(
    (context: CanvasRenderingContext2D, frame: Frame) => {
      if (domain === undefined) return;
      const palette = readPalette();
      const plotWidth = frame.width - PADDING.left - PADDING.right;
      const plotHeight = frame.height - PADDING.top - PADDING.bottom;
      if (plotWidth <= 0 || plotHeight <= 0) return;

      const x = linearScale(
        [domain.tickMin, domain.tickMax],
        [PADDING.left, PADDING.left + plotWidth],
      );
      const y = linearScale(
        [domain.valueMin, domain.valueMax],
        [PADDING.top + plotHeight, PADDING.top],
      );

      // Value grid and axis labels.
      context.font = MONO_FONT;
      context.textBaseline = "middle";
      context.textAlign = "right";
      for (const tick of niceTicks(domain.valueMin, domain.valueMax, 4)) {
        const position = Math.round(y(tick)) + 0.5;
        context.strokeStyle = tick === 0 ? palette.ruleFaint! : palette.ruleGhost!;
        context.lineWidth = 1;
        context.setLineDash(tick === 0 ? [] : [1, 3]);
        context.beginPath();
        context.moveTo(PADDING.left, position);
        context.lineTo(PADDING.left + plotWidth, position);
        context.stroke();
        context.setLineDash([]);
        context.fillStyle = palette.inkGhost!;
        context.fillText(valueFormat(tick), PADDING.left - 8, position);
      }

      // Tick axis.
      context.textAlign = "center";
      context.textBaseline = "top";
      for (const tick of niceTicks(domain.tickMin, domain.tickMax, 5)) {
        const position = Math.round(x(tick)) + 0.5;
        context.strokeStyle = palette.ruleGhost!;
        context.setLineDash([1, 3]);
        context.beginPath();
        context.moveTo(position, PADDING.top);
        context.lineTo(position, PADDING.top + plotHeight);
        context.stroke();
        context.setLineDash([]);
        context.fillStyle = palette.inkGhost!;
        context.fillText(axisLabel(tick), position, PADDING.top + plotHeight + 5);
      }

      // Plot frame: only the two measured edges, in the manner of a survey sheet.
      context.strokeStyle = palette.rule!;
      context.lineWidth = 1;
      context.beginPath();
      context.moveTo(PADDING.left + 0.5, PADDING.top);
      context.lineTo(PADDING.left + 0.5, PADDING.top + plotHeight + 0.5);
      context.lineTo(PADDING.left + plotWidth, PADDING.top + plotHeight + 0.5);
      context.stroke();

      visible.forEach((entry, index) => {
        const color = signalColor(entry.signal, palette);
        const points = entry.points;
        if (points.length === 0) return;

        if (fillFirst && index === 0 && points.length > 1) {
          context.fillStyle = hatchPattern(context, withAlpha(color, 0.45), 6);
          context.beginPath();
          context.moveTo(x(points[0]!.ticks), y(Math.max(0, domain.valueMin)));
          for (const point of points) context.lineTo(x(point.ticks), y(point.value));
          context.lineTo(x(points[points.length - 1]!.ticks), y(Math.max(0, domain.valueMin)));
          context.closePath();
          context.fill();
        }

        context.strokeStyle = color;
        context.lineWidth = 1.5;
        context.lineJoin = "round";
        context.lineCap = "round";
        context.setLineDash(entry.dashed === true ? [4, 3] : []);
        context.beginPath();
        points.forEach((point, pointIndex) => {
          const px = x(point.ticks);
          const py = y(point.value);
          if (pointIndex === 0) context.moveTo(px, py);
          else context.lineTo(px, py);
        });
        context.stroke();
        context.setLineDash([]);

        // Leading mark: the most recent received frame, ringed against the surface.
        const last = points[points.length - 1]!;
        context.fillStyle = color;
        context.strokeStyle = palette.paper!;
        context.lineWidth = 2;
        context.beginPath();
        context.arc(x(last.ticks), y(last.value), 2.75, 0, Math.PI * 2);
        context.fill();
        context.stroke();
      });

      if (probe !== undefined) {
        const px = Math.round(x(probe.ticks)) + 0.5;
        context.strokeStyle = palette.ruleBright!;
        context.setLineDash([3, 3]);
        context.lineWidth = 1;
        context.beginPath();
        context.moveTo(px, PADDING.top);
        context.lineTo(px, PADDING.top + plotHeight);
        context.stroke();
        context.setLineDash([]);
        for (const reading of probe.values) {
          const color = signalColor(reading.signal, palette);
          context.fillStyle = color;
          context.strokeStyle = palette.paper!;
          context.lineWidth = 2;
          context.beginPath();
          context.arc(px, y(reading.value), 3.5, 0, Math.PI * 2);
          context.fill();
          context.stroke();
        }
      }
    },
    [domain, fillFirst, probe, valueFormat, visible],
  );

  const onProbe = useCallback(
    (point: { x: number; y: number; frame: Frame } | undefined) => {
      if (point === undefined || domain === undefined || visible.length === 0) {
        setProbe(undefined);
        return;
      }
      const plotWidth = point.frame.width - PADDING.left - PADDING.right;
      if (plotWidth <= 0) return;
      const x = linearScale(
        [domain.tickMin, domain.tickMax],
        [PADDING.left, PADDING.left + plotWidth],
      );
      const target = x.invert(Math.max(PADDING.left, Math.min(PADDING.left + plotWidth, point.x)));
      const reference = visible[0]!.points;
      let index = 0;
      let best = Number.POSITIVE_INFINITY;
      reference.forEach((candidate, candidateIndex) => {
        const distance = Math.abs(candidate.ticks - target);
        if (distance < best) {
          best = distance;
          index = candidateIndex;
        }
      });
      const ticks = reference[index]?.ticks;
      if (ticks === undefined) return;
      const values = visible.map((entry) => {
        const match =
          entry.points.find((candidate) => candidate.ticks === ticks) ??
          entry.points[entry.points.length - 1]!;
        return { id: entry.id, signal: entry.signal, value: match.value };
      });
      const px = x(ticks);
      setProbe({
        index,
        x: px,
        ticks,
        values,
        side: px > point.frame.width * 0.6 ? "right" : "left",
        top: Math.max(8, Math.min(point.y - 12, point.frame.height - 90)),
      });
    },
    [domain, visible],
  );

  const toggle = (id: string) => {
    setMuted((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else if (current.size < series.length - 1) next.add(id);
      return next;
    });
  };

  // One sample is not a trace. Until a second frame arrives the plot would be a
  // degenerate 0..1 axis, which reads as data that was never measured.
  if (series.every((entry) => entry.points.length < 2)) {
    return <p className="chart__caption">{emptyLabel}</p>;
  }

  return (
    <div className="chart">
      {series.length > 1 && (
        <div className="chart__head">
          <div className="chart__legend">
            {series.map((entry) => (
              <button
                key={entry.id}
                type="button"
                className="chart__legend-item"
                data-muted={muted.has(entry.id)}
                aria-pressed={!muted.has(entry.id)}
                style={{ ["--signal" as string]: SIGNAL_VARIABLE[entry.signal] }}
                onClick={() => toggle(entry.id)}
              >
                <span
                  className={
                    entry.dashed === true
                      ? "chart__legend-mark chart__legend-mark--dashed"
                      : "chart__legend-mark"
                  }
                />
                {legendLabels[entry.id] ?? entry.id}
              </button>
            ))}
          </div>
        </div>
      )}

      <CanvasSurface height={height} draw={draw} label={label} onProbe={onProbe}>
        {probe !== undefined && (
          <div
            className="chart__probe"
            style={
              probe.side === "left"
                ? { left: probe.x + 12, top: probe.top }
                : { right: 12, top: probe.top }
            }
          >
            <div className="chart__probe-head">
              <span>{tickLabel}</span>
              <b className="numeric">{probe.ticks}</b>
            </div>
            {probe.values.map((reading) => (
              <div
                key={reading.id}
                className="chart__probe-row"
                style={{ ["--signal" as string]: SIGNAL_VARIABLE[reading.signal] }}
              >
                <span className="chart__probe-mark" />
                <span>{legendLabels[reading.id] ?? reading.id}</span>
                <b>{valueFormat(reading.value)}</b>
              </div>
            ))}
          </div>
        )}
      </CanvasSurface>
    </div>
  );
}

export function signalColor(signal: Series["signal"], palette: Record<string, string>): string {
  return palette[signal] ?? palette.ink!;
}

/** Canvas needs a concrete colour; the tokens are opaque hex, so alpha is applied here. */
export function withAlpha(color: string, alpha: number): string {
  if (color.startsWith("#") && color.length === 7) {
    const value = Number.parseInt(color.slice(1), 16);
    const r = (value >> 16) & 0xff;
    const g = (value >> 8) & 0xff;
    const b = value & 0xff;
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
  }
  return color;
}
