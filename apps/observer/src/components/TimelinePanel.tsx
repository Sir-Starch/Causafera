import type { Copy } from "../i18n";
import type { TimelineSample } from "../useObserverSession";

interface TimelinePanelProps {
  history: TimelineSample[];
  copy: Copy;
}

export function TimelinePanel({ history, copy }: TimelinePanelProps) {
  const manaPoints = linePoints(history.map((sample) => Number(sample.mana)), 640, 116);
  const tracePoints = linePoints(history.map((sample) => Number(sample.traces)), 640, 116);
  const latest = history[history.length - 1];

  return (
    <section className="panel timeline-panel">
      <div className="panel-heading panel-heading--row">
        <div>
          <span className="eyebrow">{copy.timeline}</span>
          <h2>{latest === undefined ? "—" : `${copy.tick} ${latest.ticks}`}</h2>
        </div>
        <div className="legend" aria-label={copy.timeline}>
          <span><i className="legend-mark legend-mark--mana" />{copy.mana}</span>
          <span><i className="legend-mark legend-mark--trace" />{copy.traces}</span>
        </div>
      </div>
      {history.length < 2 ? (
        <div className="empty-chart">{copy.needMoreSamples}</div>
      ) : (
        <svg
          className="timeline-chart"
          viewBox="0 0 640 116"
          preserveAspectRatio="none"
          role="img"
          aria-label={`${copy.mana}, ${copy.traces}`}
        >
          <line x1="0" y1="115" x2="640" y2="115" className="chart-baseline" />
          <polyline points={tracePoints} className="chart-line chart-line--trace" />
          <polyline points={manaPoints} className="chart-line chart-line--mana" />
        </svg>
      )}
    </section>
  );
}

function linePoints(values: number[], width: number, height: number): string {
  if (values.length === 0) return "";
  const minimum = Math.min(...values);
  const maximum = Math.max(...values);
  const range = maximum - minimum || 1;
  return values
    .map((value, index) => {
      const x = values.length === 1 ? 0 : (index / (values.length - 1)) * width;
      const y = height - 8 - ((value - minimum) / range) * (height - 20);
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
}
