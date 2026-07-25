/**
 * Inline trace beside a readout. SVG rather than canvas: a sparkline is a handful of points
 * and benefits from staying in the document for print and accessibility.
 */

import { useId } from "react";

import { SIGNAL_VARIABLE, SIGNAL_WASH, type SeriesPoint, type SignalId } from "../observer/models";

interface SparklineProps {
  points: readonly SeriesPoint[];
  signal: SignalId;
  height?: number;
  filled?: boolean;
  label: string;
}

const WIDTH = 100;

export function Sparkline({ points, signal, height = 28, filled = true, label }: SparklineProps) {
  const clipId = useId();
  if (points.length < 2) {
    return <svg className="sparkline" style={{ height }} aria-hidden="true" />;
  }

  const values = points.map((point) => point.value);
  const min = Math.min(...values, 0);
  const max = Math.max(...values);
  const span = max - min === 0 ? 1 : max - min;
  const first = points[0]!.ticks;
  const last = points[points.length - 1]!.ticks;
  const tickSpan = last - first === 0 ? 1 : last - first;

  const coordinates = points.map((point) => ({
    x: ((point.ticks - first) / tickSpan) * WIDTH,
    y: height - 2 - ((point.value - min) / span) * (height - 4),
  }));

  const line = coordinates
    .map((point, index) => `${index === 0 ? "M" : "L"}${point.x.toFixed(2)} ${point.y.toFixed(2)}`)
    .join(" ");
  const area = `${line} L${WIDTH} ${height} L0 ${height} Z`;
  const head = coordinates[coordinates.length - 1]!;

  return (
    <svg
      className="sparkline"
      viewBox={`0 0 ${WIDTH} ${height}`}
      preserveAspectRatio="none"
      style={{
        height,
        ["--signal" as string]: SIGNAL_VARIABLE[signal],
        ["--signal-wash" as string]: SIGNAL_WASH[signal],
      }}
      role="img"
      aria-label={label}
    >
      <clipPath id={clipId}>
        <rect x="0" y="0" width={WIDTH} height={height} />
      </clipPath>
      <g clipPath={`url(#${clipId})`}>
        {filled && <path className="sparkline__area" d={area} />}
        <path className="sparkline__line" d={line} />
      </g>
      <circle className="sparkline__head" cx={head.x} cy={head.y} r={1.6} />
    </svg>
  );
}
