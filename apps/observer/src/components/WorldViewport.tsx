import type { CSSProperties } from "react";
import type { SpatialChunkSummary, WorldChunkSnapshot } from "@causafera/observer-protocol";

import type { Copy } from "../i18n";
import { chunkKey } from "../useObserverSession";

interface WorldViewportProps {
  world?: WorldChunkSnapshot;
  selectedKey?: string;
  copy: Copy;
  onSelect(chunk: SpatialChunkSummary): void;
}

export function WorldViewport({ world, selectedKey, copy, onSelect }: WorldViewportProps) {
  if (world === undefined || world.chunks.length === 0) {
    return <div className="empty-state world-empty">{copy.noData}</div>;
  }

  const bounds = world.chunks.reduce(
    (current, chunk) => ({
      minX: Math.min(current.minX, chunk.chunkX),
      maxX: Math.max(current.maxX, chunk.chunkX),
      minY: Math.min(current.minY, chunk.chunkY),
      maxY: Math.max(current.maxY, chunk.chunkY),
    }),
    { minX: Infinity, maxX: -Infinity, minY: Infinity, maxY: -Infinity },
  );
  const elevations = world.chunks.flatMap((chunk) => [
    chunk.minimumElevationMm,
    chunk.maximumElevationMm,
  ]);
  const minElevation = Math.min(...elevations);
  const maxElevation = Math.max(...elevations);
  const maximumMana = world.chunks.reduce(
    (maximum, chunk) => (chunk.manaTotal > maximum ? chunk.manaTotal : maximum),
    1n,
  );
  const columns = bounds.maxX - bounds.minX + 1;
  const rows = bounds.maxY - bounds.minY + 1;

  return (
    <div className="world-viewport">
      <div
        className="chunk-map"
        style={{
          gridTemplateColumns: `repeat(${columns}, minmax(138px, 1fr))`,
          gridTemplateRows: `repeat(${rows}, minmax(128px, 1fr))`,
        }}
      >
        {world.chunks.map((chunk) => {
          const key = chunkKey(chunk);
          const elevation = (chunk.minimumElevationMm + chunk.maximumElevationMm) / 2;
          const elevationRatio = normalize(elevation, minElevation, maxElevation);
          const manaRatio = ratio(chunk.manaTotal, maximumMana);
          const cellStyle: CSSProperties = {
            gridColumn: chunk.chunkX - bounds.minX + 1,
            gridRow: bounds.maxY - chunk.chunkY + 1,
            backgroundColor: terrainColor(elevationRatio),
          };
          return (
            <button
              key={key}
              type="button"
              className={`chunk-cell${selectedKey === key ? " is-selected" : ""}`}
              style={cellStyle}
              aria-pressed={selectedKey === key}
              onClick={() => onSelect(chunk)}
            >
              <span className="chunk-cell__topline">
                <span className="numeric">
                  {chunk.chunkX}, {chunk.chunkY}
                </span>
                <span>R{chunk.resolutionLevel}</span>
              </span>
              <span className="chunk-cell__elevation numeric">
                {formatMeters(chunk.minimumElevationMm)}—{formatMeters(chunk.maximumElevationMm)} m
              </span>
              <span className="chunk-cell__population">
                {copy.population} <strong>{chunk.populationTotal.toString()}</strong>
              </span>
              <span className="measure-track" aria-label={`${copy.mana}: ${chunk.manaTotal}`}>
                <span style={{ width: `${Math.max(2, manaRatio * 100)}%` }} />
              </span>
            </button>
          );
        })}
      </div>

      <div className="map-footer">
        <span>{copy.boundedChart}</span>
        <span className="numeric">
          {world.chunks.length} {copy.activeChunks.toLocaleLowerCase()}
        </span>
      </div>
    </div>
  );
}

function normalize(value: number, minimum: number, maximum: number): number {
  if (maximum === minimum) return 0.5;
  return Math.max(0, Math.min(1, (value - minimum) / (maximum - minimum)));
}

function ratio(value: bigint, maximum: bigint): number {
  if (maximum <= 0n || value <= 0n) return 0;
  return Math.min(1, Number(value) / Number(maximum));
}

function terrainColor(elevationRatio: number): string {
  const hue = 36 + elevationRatio * 42;
  const lightness = 14 + elevationRatio * 8;
  return `hsl(${hue.toFixed(0)} 13% ${lightness.toFixed(0)}%)`;
}

function formatMeters(millimeters: number): string {
  return (millimeters / 1_000).toFixed(1);
}
