import type { SpatialChunkSummary } from "@causafera/observer-protocol";

import type { Copy } from "../i18n";
import { DefinitionRow } from "./DefinitionRow";

interface InspectorPanelProps {
  chunk?: SpatialChunkSummary;
  copy: Copy;
}

export function InspectorPanel({ chunk, copy }: InspectorPanelProps) {
  return (
    <section className="panel inspector-panel">
      <div className="panel-heading">
        <span className="eyebrow">{copy.objectiveProjection}</span>
        <h2>{chunk === undefined ? copy.chunk : `${copy.chunk} ${chunk.chunkX}, ${chunk.chunkY}`}</h2>
      </div>
      {chunk === undefined ? (
        <p className="muted">{copy.selectChunk}</p>
      ) : (
        <dl className="definition-list">
          <DefinitionRow term={copy.chart} value={chunk.chartId.toString()} />
          <DefinitionRow
            term={copy.elevation}
            value={`${(chunk.minimumElevationMm / 1_000).toFixed(2)} — ${(chunk.maximumElevationMm / 1_000).toFixed(2)} m`}
          />
          <DefinitionRow term={copy.roughness} value={`${chunk.meanRoughnessMm} mm`} />
          <DefinitionRow term={copy.mana} value={chunk.manaTotal.toString()} />
          <DefinitionRow term={copy.relevance} value={chunk.resolutionRelevance.toString()} />
          <DefinitionRow term={copy.population} value={chunk.populationTotal.toString()} />
          <DefinitionRow term={copy.events} value={chunk.causalEventCount.toString()} />
          <DefinitionRow term={copy.latestTrace} value={`#${chunk.latestTraceId}`} />
        </dl>
      )}
    </section>
  );
}
