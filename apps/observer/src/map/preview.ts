/**
 * Preview projections.
 *
 * Everything in this module is constructed by the observer from real values it received. None
 * of it is a measurement, and none of it invents world content — there are no settlements,
 * residents, routes or histories here, only arithmetic over chunk summaries the runtime
 * actually sent.
 *
 * It lives apart from the lens catalogue and from the renderer on purpose. A lens built on
 * anything in this file must carry `availability: "preview"`, and the interface marks it
 * wherever it appears (INV-022: a rendering is not simulation state).
 */

import type { ChunkRecord } from "../observer/models";
import type { LensIsoline, LensVector } from "./lens";
import { CHUNK_UNITS, worldOfChunk } from "./projection";

const SAMPLES_PER_CHUNK = 7;
const MARGIN_CHUNKS = 1;

interface SampleGrid {
  width: number;
  height: number;
  originX: number;
  originY: number;
  step: number;
  values: Float64Array;
  min: number;
  max: number;
}

/**
 * Inverse-distance interpolation of a per-chunk scalar onto a finer grid.
 *
 * The observer has one value per chunk and no per-cell field. The smooth surface this
 * produces is therefore a drawing convention for reading the chunk values, not terrain.
 */
function interpolate(
  chunks: readonly ChunkRecord[],
  read: (chunk: ChunkRecord) => number,
): SampleGrid | undefined {
  if (chunks.length === 0) return undefined;
  const xs = chunks.map((chunk) => chunk.chunkX);
  const ys = chunks.map((chunk) => chunk.chunkY);
  const minChunkX = Math.min(...xs) - MARGIN_CHUNKS;
  const maxChunkX = Math.max(...xs) + MARGIN_CHUNKS;
  const minChunkY = Math.min(...ys) - MARGIN_CHUNKS;
  const maxChunkY = Math.max(...ys) + MARGIN_CHUNKS;

  const width = (maxChunkX - minChunkX + 1) * SAMPLES_PER_CHUNK;
  const height = (maxChunkY - minChunkY + 1) * SAMPLES_PER_CHUNK;
  const step = CHUNK_UNITS / SAMPLES_PER_CHUNK;
  const topLeft = worldOfChunk(minChunkX, maxChunkY);
  const originX = topLeft.x - CHUNK_UNITS / 2;
  const originY = topLeft.y - CHUNK_UNITS / 2;

  const anchors = chunks.map((chunk) => {
    const world = worldOfChunk(chunk.chunkX, chunk.chunkY);
    return { x: world.x, y: world.y, value: read(chunk) };
  });

  const values = new Float64Array(width * height);
  let min = Number.POSITIVE_INFINITY;
  let max = Number.NEGATIVE_INFINITY;
  for (let row = 0; row < height; row += 1) {
    for (let column = 0; column < width; column += 1) {
      const x = originX + column * step;
      const y = originY + row * step;
      let weighted = 0;
      let weights = 0;
      for (const anchor of anchors) {
        const distance = Math.hypot(x - anchor.x, y - anchor.y);
        if (distance < 1e-3) {
          weighted = anchor.value;
          weights = 1;
          break;
        }
        const weight = 1 / (distance * distance);
        weighted += anchor.value * weight;
        weights += weight;
      }
      const value = weights === 0 ? 0 : weighted / weights;
      values[row * width + column] = value;
      if (value < min) min = value;
      if (value > max) max = value;
    }
  }
  return { width, height, originX, originY, step, values, min, max };
}

/** Marching squares over the sample grid; segments are emitted, not stitched into loops. */
function contourAt(grid: SampleGrid, threshold: number): [number, number][][] {
  const segments: [number, number][][] = [];
  const at = (column: number, row: number) => grid.values[row * grid.width + column]!;
  const point = (
    ax: number,
    ay: number,
    av: number,
    bx: number,
    by: number,
    bv: number,
  ): [number, number] => {
    const span = bv - av;
    const t = Math.abs(span) < 1e-9 ? 0.5 : (threshold - av) / span;
    return [ax + (bx - ax) * t, ay + (by - ay) * t];
  };

  for (let row = 0; row < grid.height - 1; row += 1) {
    for (let column = 0; column < grid.width - 1; column += 1) {
      const x0 = grid.originX + column * grid.step;
      const y0 = grid.originY + row * grid.step;
      const x1 = x0 + grid.step;
      const y1 = y0 + grid.step;
      const v00 = at(column, row);
      const v10 = at(column + 1, row);
      const v11 = at(column + 1, row + 1);
      const v01 = at(column, row + 1);

      const index =
        (v00 > threshold ? 8 : 0) |
        (v10 > threshold ? 4 : 0) |
        (v11 > threshold ? 2 : 0) |
        (v01 > threshold ? 1 : 0);
      if (index === 0 || index === 15) continue;

      const top = () => point(x0, y0, v00, x1, y0, v10);
      const right = () => point(x1, y0, v10, x1, y1, v11);
      const bottom = () => point(x1, y1, v11, x0, y1, v01);
      const left = () => point(x0, y1, v01, x0, y0, v00);

      switch (index) {
        case 1:
        case 14:
          segments.push([left(), bottom()]);
          break;
        case 2:
        case 13:
          segments.push([bottom(), right()]);
          break;
        case 3:
        case 12:
          segments.push([left(), right()]);
          break;
        case 4:
        case 11:
          segments.push([top(), right()]);
          break;
        case 6:
        case 9:
          segments.push([top(), bottom()]);
          break;
        case 7:
        case 8:
          segments.push([left(), top()]);
          break;
        case 5:
          segments.push([left(), top()], [bottom(), right()]);
          break;
        case 10:
          segments.push([left(), bottom()], [top(), right()]);
          break;
        default:
          break;
      }
    }
  }
  return segments;
}

/** Isolines over an interpolated chunk scalar, at evenly spaced levels. */
export function previewIsolines(
  chunks: readonly ChunkRecord[],
  read: (chunk: ChunkRecord) => number,
  levels = 9,
  format?: (value: number) => string,
): LensIsoline[] {
  const grid = interpolate(chunks, read);
  if (grid === undefined || grid.max - grid.min < 1e-6) return [];
  const lines: LensIsoline[] = [];
  for (let index = 1; index <= levels; index += 1) {
    const fraction = index / (levels + 1);
    const threshold = grid.min + (grid.max - grid.min) * fraction;
    for (const segment of contourAt(grid, threshold)) {
      lines.push({
        points: segment,
        level: fraction,
        label: format === undefined ? undefined : format(threshold),
      });
    }
  }
  return lines;
}

/**
 * Differences between neighbouring chunks, drawn as arrows.
 *
 * This is a difference, not a measured flux: the observer receives no transport term between
 * chunks. The arrow points from the lower value to the higher one.
 */
export function previewGradient(
  chunks: readonly ChunkRecord[],
  read: (chunk: ChunkRecord) => number,
  format: (value: number) => string,
): LensVector[] {
  const byPosition = new Map<string, ChunkRecord>();
  for (const chunk of chunks) byPosition.set(`${chunk.chunkX}:${chunk.chunkY}`, chunk);

  const vectors: { from: ChunkRecord; to: ChunkRecord; delta: number }[] = [];
  for (const chunk of chunks) {
    for (const [dx, dy] of [
      [1, 0],
      [0, 1],
    ] as const) {
      const neighbour = byPosition.get(`${chunk.chunkX + dx}:${chunk.chunkY + dy}`);
      if (neighbour === undefined) continue;
      const delta = read(neighbour) - read(chunk);
      if (delta === 0) continue;
      vectors.push(
        delta > 0
          ? { from: chunk, to: neighbour, delta }
          : { from: neighbour, to: chunk, delta: -delta },
      );
    }
  }
  if (vectors.length === 0) return [];
  const strongest = Math.max(...vectors.map((vector) => vector.delta));
  return vectors.map((vector) => {
    const from = worldOfChunk(vector.from.chunkX, vector.from.chunkY);
    const to = worldOfChunk(vector.to.chunkX, vector.to.chunkY);
    return {
      fromX: from.x,
      fromY: from.y,
      toX: to.x,
      toY: to.y,
      weight: vector.delta / strongest,
      label: format(vector.delta),
    };
  });
}
