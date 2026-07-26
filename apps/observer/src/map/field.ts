/**
 * Continuous fields over the chart.
 *
 * The observer receives one lattice per chunk. Drawing each of those on its own
 * is what made the map a grid of squares: every chunk got its own normalisation,
 * its own edge, and a visible seam wherever two of them met. This module does
 * the opposite — it assembles every received lattice into one field over the
 * whole charted extent, and everything downstream reads that: the tinting, the
 * relief shading, the contours and the hover readout all sample one surface.
 *
 * Three rules keep this a reading of measurements rather than an invention.
 *
 * 1. **Only received samples enter.** Ground the observer has not been given is
 *    marked uncovered and drawn as unsurveyed, never interpolated across.
 * 2. **Interpolation is stated.** Between samples the field is resampled with a
 *    Catmull-Rom kernel, which passes exactly through every measured sample and
 *    reaches no value outside the local range once clamped. It is a way of
 *    reading a sampled field, and the lenses that use it say so.
 * 3. **Nothing computed here returns to the runtime.** Shading is a lighting
 *    choice and lighting is presentation (INV-022).
 */

import { CHUNK_UNITS } from "./projection";

/** One chunk's plan-view lattice, already reduced from whatever it arrived as. */
export interface FieldPatch {
  chunkX: number;
  chunkY: number;
  /** Samples along one edge. Every patch in a field must agree. */
  edge: number;
  /** `edge²` samples, row-major, row 0 southmost. */
  values: Float64Array;
}

/**
 * One scalar field over a rectangular box of chunks.
 *
 * Rows run south to north so that a rising row index is a rising chart y, which
 * is the same orientation the cell marks already use.
 */
export interface ChartField {
  originChunkX: number;
  originChunkY: number;
  chunksX: number;
  chunksY: number;
  edge: number;
  width: number;
  height: number;
  values: Float64Array;
  /** 1 where a received patch supplied the sample, 0 where nothing was received. */
  covered: Uint8Array;
  min: number;
  max: number;
  /** True when every sample is the same value, so `min` and `max` coincide. */
  uniform: boolean;
  /** How many chunks contributed. A field of one chunk is still a field. */
  patches: number;
}

/** Assemble received patches into one field. Patches of a foreign edge are refused. */
export function assembleField(patches: readonly FieldPatch[]): ChartField | undefined {
  const first = patches[0];
  if (first === undefined) return undefined;
  const edge = first.edge;
  const usable = patches.filter(
    (patch) => patch.edge === edge && patch.values.length === edge * edge,
  );
  if (usable.length === 0) return undefined;

  const originChunkX = Math.min(...usable.map((patch) => patch.chunkX));
  const originChunkY = Math.min(...usable.map((patch) => patch.chunkY));
  const chunksX = Math.max(...usable.map((patch) => patch.chunkX)) - originChunkX + 1;
  const chunksY = Math.max(...usable.map((patch) => patch.chunkY)) - originChunkY + 1;
  const width = chunksX * edge;
  const height = chunksY * edge;
  const values = new Float64Array(width * height);
  const covered = new Uint8Array(width * height);

  let min = Number.POSITIVE_INFINITY;
  let max = Number.NEGATIVE_INFINITY;
  for (const patch of usable) {
    const offsetX = (patch.chunkX - originChunkX) * edge;
    const offsetY = (patch.chunkY - originChunkY) * edge;
    for (let row = 0; row < edge; row += 1) {
      for (let column = 0; column < edge; column += 1) {
        const value = patch.values[row * edge + column]!;
        const index = (offsetY + row) * width + offsetX + column;
        values[index] = value;
        covered[index] = 1;
        if (value < min) min = value;
        if (value > max) max = value;
      }
    }
  }
  if (min > max) return undefined;
  return {
    originChunkX,
    originChunkY,
    chunksX,
    chunksY,
    edge,
    width,
    height,
    values,
    covered,
    min,
    max,
    // Consumers guard the zero span themselves rather than being handed a padded
    // one: the legend of an empty field should read as the value it holds, not
    // as a range it does not have.
    uniform: max === min,
    patches: usable.length,
  };
}

/* ------------------------------------------------------------- sampling -- */

/** World position of a sample centre, in chart units. */
export function worldOfSample(
  field: ChartField,
  sampleX: number,
  sampleY: number,
): { x: number; y: number } {
  const step = CHUNK_UNITS / field.edge;
  return {
    x: (field.originChunkX - 0.5) * CHUNK_UNITS + (sampleX + 0.5) * step,
    y: -((field.originChunkY - 0.5) * CHUNK_UNITS + (sampleY + 0.5) * step),
  };
}

/** Sample-space position of a world point. Fractional between sample centres. */
export function sampleOfWorld(
  field: ChartField,
  worldX: number,
  worldY: number,
): { x: number; y: number } {
  return {
    x: (worldX / CHUNK_UNITS + 0.5 - field.originChunkX) * field.edge - 0.5,
    y: (-worldY / CHUNK_UNITS + 0.5 - field.originChunkY) * field.edge - 0.5,
  };
}

function at(field: ChartField, column: number, row: number): number {
  const x = column < 0 ? 0 : column >= field.width ? field.width - 1 : column;
  const y = row < 0 ? 0 : row >= field.height ? field.height - 1 : row;
  return field.values[y * field.width + x]!;
}

export function isCovered(field: ChartField, column: number, row: number): boolean {
  if (column < 0 || row < 0 || column >= field.width || row >= field.height) return false;
  return field.covered[row * field.width + column] === 1;
}

function catmullRom(a: number, b: number, c: number, d: number, t: number): number {
  const value =
    b + 0.5 * t * (c - a + t * (2 * a - 5 * b + 4 * c - d + t * (3 * (b - c) + d - a)));
  // A cubic through four samples can overshoot; clamping to the bracketing pair
  // keeps the drawn field inside the range the measurements actually span.
  const low = b < c ? b : c;
  const high = b < c ? c : b;
  return value < low ? low : value > high ? high : value;
}

/**
 * The field at a fractional sample position.
 *
 * Catmull-Rom passes through every measured sample exactly, so a sample position
 * that lands on a measurement returns that measurement and nothing else.
 */
export function sampleField(field: ChartField, sampleX: number, sampleY: number): number {
  const column = Math.floor(sampleX);
  const row = Math.floor(sampleY);
  const tx = sampleX - column;
  const ty = sampleY - row;
  const rows: number[] = [];
  for (let offset = -1; offset <= 2; offset += 1) {
    rows.push(
      catmullRom(
        at(field, column - 1, row + offset),
        at(field, column, row + offset),
        at(field, column + 1, row + offset),
        at(field, column + 2, row + offset),
        tx,
      ),
    );
  }
  return catmullRom(rows[0]!, rows[1]!, rows[2]!, rows[3]!, ty);
}

/** The field under a world point, or `undefined` over unsurveyed ground. */
export function sampleFieldAtWorld(
  field: ChartField,
  worldX: number,
  worldY: number,
): number | undefined {
  const position = sampleOfWorld(field, worldX, worldY);
  if (!isCovered(field, Math.round(position.x), Math.round(position.y))) return undefined;
  return sampleField(field, position.x, position.y);
}

/** Position of a value within the field's own range, clamped to 0..1. */
export function normalise(field: ChartField, value: number): number {
  const span = field.max - field.min;
  const t = span === 0 ? 0 : (value - field.min) / span;
  return t < 0 ? 0 : t > 1 ? 1 : t;
}

/* ------------------------------------------------------------- coverage -- */

/**
 * Distance in samples from each covered sample to the nearest uncovered one.
 *
 * The rendered surface uses it to resolve its own edge over a sample or two
 * rather than as a hard cut, so the survey boundary reads as an engraved edge
 * instead of a pasted rectangle. The boundary itself is still drawn.
 */
export function coverageDistance(field: ChartField): Float32Array {
  const { width, height, covered } = field;
  const distance = new Float32Array(width * height);
  const far = width + height;
  for (let index = 0; index < distance.length; index += 1) {
    distance[index] = covered[index] === 1 ? far : 0;
  }
  const relax = (index: number, other: number, cost: number) => {
    const candidate = distance[other]! + cost;
    if (candidate < distance[index]!) distance[index] = candidate;
  };
  for (let row = 0; row < height; row += 1) {
    for (let column = 0; column < width; column += 1) {
      const index = row * width + column;
      if (distance[index] === 0) continue;
      // Outside the lattice counts as uncovered, so the outer edge feathers too.
      if (column === 0 || row === 0) distance[index] = Math.min(distance[index]!, 1);
      if (column > 0) relax(index, index - 1, 1);
      if (row > 0) relax(index, index - width, 1);
      if (column > 0 && row > 0) relax(index, index - width - 1, Math.SQRT2);
    }
  }
  for (let row = height - 1; row >= 0; row -= 1) {
    for (let column = width - 1; column >= 0; column -= 1) {
      const index = row * width + column;
      if (distance[index] === 0) continue;
      if (column === width - 1 || row === height - 1) {
        distance[index] = Math.min(distance[index]!, 1);
      }
      if (column + 1 < width) relax(index, index + 1, 1);
      if (row + 1 < height) relax(index, index + width, 1);
      if (column + 1 < width && row + 1 < height) relax(index, index + width + 1, Math.SQRT2);
    }
  }
  return distance;
}

/* -------------------------------------------------------------- shading -- */

export interface ShadingOptions {
  /** Degrees clockwise from north. The cartographic convention is 315. */
  azimuth: number;
  /** Degrees above the horizon. */
  altitude: number;
  /**
   * Vertical exaggeration, applied to the field normalised into 0..1. It is a
   * presentation parameter and has no unit in the measured field.
   */
  exaggeration: number;
}

export const DEFAULT_SHADING: ShadingOptions = {
  azimuth: 315,
  altitude: 45,
  exaggeration: 2.4,
};

/**
 * A Lambertian shading term for the field, in 0..1, at a fractional sample
 * position. Computed from the same interpolant the tint uses, so relief and
 * colour cannot disagree about where a slope is.
 */
export function shadeAt(
  field: ChartField,
  sampleX: number,
  sampleY: number,
  options: ShadingOptions,
): number {
  const span = field.max - field.min || 1;
  const step = 0.75;
  const west = sampleField(field, sampleX - step, sampleY);
  const east = sampleField(field, sampleX + step, sampleY);
  const south = sampleField(field, sampleX, sampleY - step);
  const north = sampleField(field, sampleX, sampleY + step);
  const dzdx = (((east - west) / span) * options.exaggeration) / (2 * step);
  const dzdy = (((north - south) / span) * options.exaggeration) / (2 * step);

  const zenith = ((90 - options.altitude) * Math.PI) / 180;
  const azimuth = ((360 - options.azimuth + 90) * Math.PI) / 180;
  const slope = Math.atan(Math.hypot(dzdx, dzdy));
  const aspect = Math.atan2(dzdy, -dzdx);
  const shade =
    Math.cos(zenith) * Math.cos(slope) +
    Math.sin(zenith) * Math.sin(slope) * Math.cos(azimuth - aspect);
  return shade < 0 ? 0 : shade > 1 ? 1 : shade;
}

/**
 * The same field resampled onto a denser lattice.
 *
 * A contour traced on a coarse lattice is a polygon, and drawn over a surface
 * painted from the smooth interpolant it visibly disagrees with the tint beneath
 * it. Refining first makes the line follow the same interpolation the surface is
 * painted with, so the two are readings of one field rather than two.
 *
 * Nothing is invented: every original sample survives at its own position,
 * because the interpolant passes through it exactly.
 */
export function refineField(field: ChartField, factor: number): ChartField {
  if (factor <= 1) return field;
  const edge = field.edge * factor;
  const width = field.chunksX * edge;
  const height = field.chunksY * edge;
  const values = new Float64Array(width * height);
  const covered = new Uint8Array(width * height);
  for (let row = 0; row < height; row += 1) {
    const sampleY = (row + 0.5) / factor - 0.5;
    for (let column = 0; column < width; column += 1) {
      const sampleX = (column + 0.5) / factor - 0.5;
      const index = row * width + column;
      // A refined sample only exists where the measurement under it does.
      if (!isCovered(field, Math.round(sampleX), Math.round(sampleY))) continue;
      values[index] = sampleField(field, sampleX, sampleY);
      covered[index] = 1;
    }
  }
  return { ...field, edge, width, height, values, covered };
}

/* ------------------------------------------------------------- contours -- */

export interface ContourLine {
  /** The measured value this line follows. */
  value: number;
  /** Position of the value within the field's range, for line weight. */
  level: number;
  /** Index of the value within the contour set, so every fifth can be weighted. */
  ordinal: number;
  /** One chart-space polyline. Every vertex sits between two measured samples. */
  points: [number, number][];
}

/**
 * Contours by marching squares over the assembled samples.
 *
 * Every vertex is a linear interpolation between two adjacent measurements, and
 * a cell touching unsurveyed ground contributes nothing, so a contour never
 * crosses ground the observer was not given.
 */
export function contourLines(field: ChartField, levels: readonly number[]): ContourLine[] {
  const lines: ContourLine[] = [];
  const span = field.max - field.min || 1;
  for (const [ordinal, value] of levels.entries()) {
    const points: [number, number][] = [];
    for (let row = 0; row + 1 < field.height; row += 1) {
      for (let column = 0; column + 1 < field.width; column += 1) {
        if (
          !isCovered(field, column, row) ||
          !isCovered(field, column + 1, row) ||
          !isCovered(field, column, row + 1) ||
          !isCovered(field, column + 1, row + 1)
        ) {
          continue;
        }
        const southWest = at(field, column, row);
        const southEast = at(field, column + 1, row);
        const northWest = at(field, column, row + 1);
        const northEast = at(field, column + 1, row + 1);
        let code = 0;
        if (southWest > value) code |= 1;
        if (southEast > value) code |= 2;
        if (northEast > value) code |= 4;
        if (northWest > value) code |= 8;
        if (code === 0 || code === 15) continue;

        const south = (): [number, number] =>
          edgePoint(field, column, row, column + 1, row, southWest, southEast, value);
        const east = (): [number, number] =>
          edgePoint(field, column + 1, row, column + 1, row + 1, southEast, northEast, value);
        const north = (): [number, number] =>
          edgePoint(field, column, row + 1, column + 1, row + 1, northWest, northEast, value);
        const west = (): [number, number] =>
          edgePoint(field, column, row, column, row + 1, southWest, northWest, value);

        // Saddles are resolved by the cell mean, which is the standard choice
        // and keeps the two branches from crossing.
        const mean = (southWest + southEast + northWest + northEast) / 4;
        switch (code) {
          case 1:
          case 14:
            points.push(west(), south());
            break;
          case 2:
          case 13:
            points.push(south(), east());
            break;
          case 3:
          case 12:
            points.push(west(), east());
            break;
          case 4:
          case 11:
            points.push(east(), north());
            break;
          case 6:
          case 9:
            points.push(south(), north());
            break;
          case 7:
          case 8:
            points.push(west(), north());
            break;
          case 5:
            if (mean > value) {
              points.push(west(), north(), south(), east());
            } else {
              points.push(west(), south(), north(), east());
            }
            break;
          default:
            if (mean > value) {
              points.push(south(), west(), east(), north());
            } else {
              points.push(south(), east(), west(), north());
            }
            break;
        }
      }
    }
    for (const polyline of chain(points)) {
      lines.push({ value, level: (value - field.min) / span, ordinal, points: polyline });
    }
  }
  return lines;
}

/**
 * Join marching-squares segments into polylines.
 *
 * The cells are visited in scan order, so the segments of one contour arrive
 * scattered. Stroking them individually costs a path per segment and leaves
 * visible breaks at every join; walking them into polylines first makes a
 * contour one line, which is also what lets it carry a label.
 */
function chain(segments: readonly [number, number][]): [number, number][][] {
  const key = (point: [number, number]): string =>
    `${Math.round(point[0] * 64)}|${Math.round(point[1] * 64)}`;
  const ends = new Map<string, number[]>();
  const used = new Array(segments.length / 2).fill(false) as boolean[];
  const attach = (at: string, segment: number) => {
    const found = ends.get(at);
    if (found === undefined) ends.set(at, [segment]);
    else found.push(segment);
  };
  for (let segment = 0; segment * 2 + 1 < segments.length; segment += 1) {
    attach(key(segments[segment * 2]!), segment);
    attach(key(segments[segment * 2 + 1]!), segment);
  }

  const step = (from: string, exclude: number): { segment: number; to: [number, number] } | undefined => {
    for (const segment of ends.get(from) ?? []) {
      if (segment === exclude || used[segment] === true) continue;
      const a = segments[segment * 2]!;
      const b = segments[segment * 2 + 1]!;
      return { segment, to: key(a) === from ? b : a };
    }
    return undefined;
  };

  const polylines: [number, number][][] = [];
  for (let segment = 0; segment * 2 + 1 < segments.length; segment += 1) {
    if (used[segment] === true) continue;
    used[segment] = true;
    const a = segments[segment * 2]!;
    const b = segments[segment * 2 + 1]!;
    const line: [number, number][] = [a, b];
    let head = key(b);
    for (;;) {
      const next = step(head, -1);
      if (next === undefined) break;
      used[next.segment] = true;
      line.push(next.to);
      head = key(next.to);
    }
    let tail = key(a);
    for (;;) {
      const next = step(tail, -1);
      if (next === undefined) break;
      used[next.segment] = true;
      line.unshift(next.to);
      tail = key(next.to);
    }
    polylines.push(line);
  }
  return polylines;
}

function edgePoint(
  field: ChartField,
  columnA: number,
  rowA: number,
  columnB: number,
  rowB: number,
  valueA: number,
  valueB: number,
  level: number,
): [number, number] {
  const denominator = valueB - valueA;
  const t = denominator === 0 ? 0.5 : (level - valueA) / denominator;
  const clamped = t < 0 ? 0 : t > 1 ? 1 : t;
  const a = worldOfSample(field, columnA, rowA);
  const b = worldOfSample(field, columnB, rowB);
  return [a.x + (b.x - a.x) * clamped, a.y + (b.y - a.y) * clamped];
}

/**
 * Contour values at a round interval covering the field.
 *
 * The interval is chosen from the range so a chart of seventy metres of relief
 * gets ten-metre contours rather than an arbitrary fraction of its own span —
 * a contour interval is a stated quantity, not a normalised one.
 */
export function contourLevels(field: ChartField, target: number, unit = 1): number[] {
  const span = (field.max - field.min) / unit;
  if (span <= 0) return [];
  const raw = span / Math.max(2, target);
  const magnitude = 10 ** Math.floor(Math.log10(raw));
  const normalised = raw / magnitude;
  const step =
    (normalised >= 5 ? 5 : normalised >= 2.5 ? 5 : normalised >= 2 ? 2 : normalised >= 1.5 ? 2 : 1) *
    magnitude *
    unit;
  const levels: number[] = [];
  const first = Math.ceil(field.min / step) * step;
  for (let value = first; value < field.max; value += step) {
    if (value > field.min) levels.push(value);
    if (levels.length > 64) break;
  }
  return levels;
}
