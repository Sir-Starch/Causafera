/**
 * Chart projection.
 *
 * The world is a set of charts; a chart is a lattice of chunks addressed by integer
 * coordinates. There is no seamless global Cartesian surface (INV-036), so the map projects
 * exactly one chart at one containment layer and states which. Everything outside the
 * received extent is unsurveyed ground, not empty space.
 *
 * World units are chart units: one chunk is `CHUNK_UNITS` square. The viewport carries a
 * centre in world units and a scale in pixels per world unit, which makes zoom anchoring and
 * level-of-detail thresholds a single multiplication rather than a special case per layer.
 */

import { CHUNK_SIZE, type ChunkRecord } from "../observer/models";

export const CHUNK_UNITS = 96;
export const CELLS_PER_EDGE = CHUNK_SIZE;

export const MIN_SCALE = 0.06;
export const MAX_SCALE = 9;

/** Pixels a single chunk occupies at the current scale — the basis of every LOD decision. */
export function chunkPixels(scale: number): number {
  return CHUNK_UNITS * scale;
}

export type DetailLevel = "field" | "chunk" | "cell";

/**
 * Detail follows legibility, not preference: a chunk smaller than a glyph gets no glyphs, and
 * the cell lattice only appears once a cell is large enough to carry a mark.
 */
export function detailFor(scale: number): DetailLevel {
  const pixels = chunkPixels(scale);
  if (pixels < 30) return "field";
  if (pixels < 190) return "chunk";
  return "cell";
}

export interface Viewport {
  /** Centre of the view, in world units. */
  x: number;
  y: number;
  scale: number;
}

export interface Screen {
  width: number;
  height: number;
}

/** North is up: chart Y increases northward, screen Y increases downward. */
export function worldOfChunk(chunkX: number, chunkY: number): { x: number; y: number } {
  return { x: chunkX * CHUNK_UNITS, y: -chunkY * CHUNK_UNITS };
}

export function toScreen(
  viewport: Viewport,
  screen: Screen,
  worldX: number,
  worldY: number,
): { x: number; y: number } {
  return {
    x: (worldX - viewport.x) * viewport.scale + screen.width / 2,
    y: (worldY - viewport.y) * viewport.scale + screen.height / 2,
  };
}

export function toWorld(
  viewport: Viewport,
  screen: Screen,
  screenX: number,
  screenY: number,
): { x: number; y: number } {
  return {
    x: (screenX - screen.width / 2) / viewport.scale + viewport.x,
    y: (screenY - screen.height / 2) / viewport.scale + viewport.y,
  };
}

/** The chunk under a world point, whether or not the observer has surveyed it. */
export function chunkAt(worldX: number, worldY: number): { chunkX: number; chunkY: number } {
  return {
    chunkX: Math.floor(worldX / CHUNK_UNITS + 0.5),
    chunkY: Math.floor(-worldY / CHUNK_UNITS + 0.5),
  };
}

/** The cell inside a chunk under a world point, in local lattice coordinates. */
export function cellAt(worldX: number, worldY: number): { cellX: number; cellY: number } {
  const unit = CHUNK_UNITS / CELLS_PER_EDGE;
  const chunk = chunkAt(worldX, worldY);
  const originX = chunk.chunkX * CHUNK_UNITS - CHUNK_UNITS / 2;
  const originY = -chunk.chunkY * CHUNK_UNITS - CHUNK_UNITS / 2;
  return {
    cellX: clampCell(Math.floor((worldX - originX) / unit)),
    cellY: clampCell(CELLS_PER_EDGE - 1 - Math.floor((worldY - originY) / unit)),
  };
}

function clampCell(value: number): number {
  return Math.max(0, Math.min(CELLS_PER_EDGE - 1, value));
}

export interface ChartExtent {
  minX: number;
  maxX: number;
  minY: number;
  maxY: number;
}

/** The charted extent: the chunk box the observer has actually received. */
export function extentOf(chunks: readonly ChunkRecord[]): ChartExtent | undefined {
  if (chunks.length === 0) return undefined;
  return {
    minX: Math.min(...chunks.map((chunk) => chunk.chunkX)),
    maxX: Math.max(...chunks.map((chunk) => chunk.chunkX)),
    minY: Math.min(...chunks.map((chunk) => chunk.chunkY)),
    maxY: Math.max(...chunks.map((chunk) => chunk.chunkY)),
  };
}

/** A viewport that frames the charted extent with a margin of unsurveyed ground around it. */
export function fitViewport(
  extent: ChartExtent | undefined,
  screen: Screen,
  marginChunks = 1.6,
): Viewport {
  if (extent === undefined || screen.width === 0 || screen.height === 0) {
    return { x: 0, y: 0, scale: 1 };
  }
  const spanX = (extent.maxX - extent.minX + 1 + marginChunks * 2) * CHUNK_UNITS;
  const spanY = (extent.maxY - extent.minY + 1 + marginChunks * 2) * CHUNK_UNITS;
  const scale = clampScale(Math.min(screen.width / spanX, screen.height / spanY));
  const centre = worldOfChunk(
    (extent.minX + extent.maxX) / 2,
    (extent.minY + extent.maxY) / 2,
  );
  return { x: centre.x, y: centre.y, scale };
}

export function clampScale(scale: number): number {
  return Math.max(MIN_SCALE, Math.min(MAX_SCALE, scale));
}

/** Zoom about a fixed screen point, so the world under the cursor stays under the cursor. */
export function zoomAt(
  viewport: Viewport,
  screen: Screen,
  screenX: number,
  screenY: number,
  factor: number,
): Viewport {
  const before = toWorld(viewport, screen, screenX, screenY);
  const scale = clampScale(viewport.scale * factor);
  const after = toWorld({ ...viewport, scale }, screen, screenX, screenY);
  return { x: viewport.x + (before.x - after.x), y: viewport.y + (before.y - after.y), scale };
}

/**
 * Chunk coordinates covering the viewport, padded by one ring.
 *
 * Culling is written for a chart far larger than the three chunks the runtime currently
 * projects: the renderer must never iterate a whole chart to draw a screenful of it.
 */
export function visibleChunks(viewport: Viewport, screen: Screen): ChartExtent {
  const topLeft = toWorld(viewport, screen, 0, 0);
  const bottomRight = toWorld(viewport, screen, screen.width, screen.height);
  const a = chunkAt(topLeft.x, topLeft.y);
  const b = chunkAt(bottomRight.x, bottomRight.y);
  return {
    minX: Math.min(a.chunkX, b.chunkX) - 1,
    maxX: Math.max(a.chunkX, b.chunkX) + 1,
    minY: Math.min(a.chunkY, b.chunkY) - 1,
    maxY: Math.max(a.chunkY, b.chunkY) + 1,
  };
}

/** Graticule spacing in chunks, chosen so labelled rules stay roughly a screen apart. */
export function graticuleStep(scale: number): number {
  const pixels = chunkPixels(scale);
  if (pixels > 150) return 1;
  if (pixels > 60) return 2;
  if (pixels > 24) return 4;
  if (pixels > 9) return 8;
  return 16;
}
