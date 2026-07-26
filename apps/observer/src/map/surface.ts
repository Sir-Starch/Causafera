/**
 * Painting a continuous field.
 *
 * The chart draws a field as one image over the whole surveyed extent rather
 * than as a fill per chunk. That is the difference between a map and a grid of
 * squares: the tint, the relief and the contours all come from the same
 * assembled surface, so a gradient that crosses a chunk boundary is drawn as one
 * gradient and a discontinuity in the drawing is a discontinuity in the world.
 *
 * Colour is measurement here. The hypsometric ramp is a sequential scale over
 * elevation and the single-hue ramp is a sequential scale over intensity;
 * neither carries decoration, and the legend states the range each covers.
 */

import {
  DEFAULT_SHADING,
  coverageDistance,
  normalise,
  sampleField,
  shadeAt,
  type ChartField,
  type ShadingOptions,
} from "./field";
import { CHUNK_UNITS } from "./projection";

export type RampStop = readonly [number, number, number, number];

/**
 * A hypsometric scale for measured relief.
 *
 * Cold and dark at the floor, slate through the middle ground, bone at the
 * crests. Chroma stays low throughout: this is a night sheet, and a rainbow
 * would read as decoration rather than as elevation.
 */
export const RELIEF_RAMP: readonly RampStop[] = [
  [0.0, 0x0b, 0x16, 0x1c],
  [0.16, 0x13, 0x21, 0x28],
  [0.34, 0x1c, 0x2b, 0x2c],
  [0.52, 0x2a, 0x34, 0x2c],
  [0.68, 0x43, 0x43, 0x31],
  [0.85, 0x6d, 0x61, 0x41],
  [1.0, 0x9d, 0x90, 0x75],
];

/**
 * The single-hue sequential ramp from the tokens, as a field scale.
 *
 * It starts near the paper rather than at a saturated dark, so quiet ground
 * reads as quiet ground instead of as a brown wash over the whole sheet.
 */
export const MANA_RAMP: readonly RampStop[] = [
  [0.0, 0x10, 0x11, 0x12],
  [0.2, 0x3a, 0x2a, 0x0d],
  [0.45, 0x6f, 0x4c, 0x11],
  [0.68, 0xa8, 0x74, 0x13],
  [0.85, 0xc9, 0x93, 0x2a],
  [0.95, 0xdf, 0xb8, 0x71],
  [1.0, 0xf2, 0xe2, 0xc4],
];

/**
 * A neutral ink ramp for a measured surface property that is not a height and
 * carries no hue of its own.
 */
export const TEXTURE_RAMP: readonly RampStop[] = [
  [0.0, 0x0a, 0x0f, 0x12],
  [0.35, 0x22, 0x2c, 0x31],
  [0.7, 0x4d, 0x5a, 0x60],
  [1.0, 0x9d, 0xab, 0xb1],
];

export interface SurfaceStyle {
  ramp: readonly RampStop[];
  /** Relief shading, or `false` for a field whose gradient is not a slope. */
  shading: ShadingOptions | false;
  /** Opacity at the bottom and the top of the ramp. */
  alpha: readonly [number, number];
  /** Gamma on the normalised value before the ramp; 1 leaves the scale linear. */
  gamma?: number;
}

export const RELIEF_STYLE: SurfaceStyle = {
  ramp: RELIEF_RAMP,
  shading: DEFAULT_SHADING,
  alpha: [1, 1],
};

export const TEXTURE_STYLE: SurfaceStyle = {
  ramp: TEXTURE_RAMP,
  shading: false,
  alpha: [0.72, 0.96],
};

export const MANA_STYLE: SurfaceStyle = {
  ramp: MANA_RAMP,
  // Mana intensity is not a height, so a slope through it is not a hillside.
  shading: false,
  alpha: [0.06, 0.96],
  // The field spans nearly three orders of magnitude across the chart. A mild
  // gamma keeps the quiet ground legible; a stronger one lifted the whole sheet
  // into the middle of the ramp and lost the structure it was meant to show.
  gamma: 0.65,
};

/** The chart-space box the field covers, in world units. */
export function fieldBounds(field: ChartField): {
  west: number;
  east: number;
  north: number;
  south: number;
} {
  const west = (field.originChunkX - 0.5) * CHUNK_UNITS;
  const east = west + field.chunksX * CHUNK_UNITS;
  const south = -((field.originChunkY - 0.5) * CHUNK_UNITS);
  const north = south - field.chunksY * CHUNK_UNITS;
  return { west, east, north, south };
}

/**
 * Texels per chunk edge.
 *
 * A coarse lattice needs heavy upsampling to read as a field at all, and a fine
 * one needs enough texels to keep its own detail; both are capped so the surface
 * stays a few hundred kilobytes however large the chart grows.
 */
export function textureDensity(field: ChartField): number {
  const wanted = field.edge * 4;
  const capped = Math.min(160, Math.max(96, wanted));
  const totalCap = 2048;
  const longest = Math.max(field.chunksX, field.chunksY);
  return Math.max(24, Math.min(capped, Math.floor(totalCap / longest)));
}

function rampColour(ramp: readonly RampStop[], t: number): [number, number, number] {
  const clamped = t < 0 ? 0 : t > 1 ? 1 : t;
  let upper = 1;
  while (upper < ramp.length - 1 && ramp[upper]![0] < clamped) upper += 1;
  const low = ramp[upper - 1]!;
  const high = ramp[upper]!;
  const span = high[0] - low[0];
  const local = span === 0 ? 0 : (clamped - low[0]) / span;
  return [
    low[1] + (high[1] - low[1]) * local,
    low[2] + (high[2] - low[2]) * local,
    low[3] + (high[3] - low[3]) * local,
  ];
}

/**
 * The field as pixels.
 *
 * Resolved in sample space rather than screen space, so the cost is a property
 * of how much ground has been surveyed and not of how far the chart is zoomed
 * in; the canvas scales the result.
 */
export function renderSurface(
  field: ChartField,
  style: SurfaceStyle,
  create: (width: number, height: number) => ImageData,
): ImageData {
  const density = textureDensity(field);
  const width = field.chunksX * density;
  const height = field.chunksY * density;
  const image = create(width, height);
  const pixels = image.data;
  const distance = coverageDistance(field);
  // Resolve the survey edge over a sample, so the boundary reads as an engraved
  // edge rather than a pasted rectangle. The boundary itself is still drawn.
  const feather = 0.9;
  const gamma = style.gamma ?? 1;

  for (let row = 0; row < height; row += 1) {
    // Texture rows run north to south; the field's rows run south to north.
    const sampleY = (1 - (row + 0.5) / height) * field.height - 0.5;
    for (let column = 0; column < width; column += 1) {
      const sampleX = ((column + 0.5) / width) * field.width - 0.5;
      const target = (row * width + column) * 4;

      const cover = bilinear(distance, field.width, field.height, sampleX, sampleY);
      if (cover <= 0) {
        pixels[target + 3] = 0;
        continue;
      }

      const value = sampleField(field, sampleX, sampleY);
      // A field that holds one value everywhere has no range to place it in.
      // Drawing it at the floor would read as absence; it is drawn at the middle
      // of the ramp, and the legend states that both extremes are that value.
      const t = field.uniform ? 0.5 : normalise(field, value);
      const scaled = gamma === 1 ? t : t ** gamma;
      const [red, green, blue] = rampColour(style.ramp, scaled);
      const shade =
        style.shading === false ? 1 : 0.42 + shadeAt(field, sampleX, sampleY, style.shading) * 0.92;

      const alpha =
        (style.alpha[0] + (style.alpha[1] - style.alpha[0]) * scaled) *
        Math.min(1, cover / feather);
      pixels[target] = clampByte(red * shade);
      pixels[target + 1] = clampByte(green * shade);
      pixels[target + 2] = clampByte(blue * shade);
      pixels[target + 3] = clampByte(alpha * 255);
    }
  }
  return image;
}

function bilinear(
  values: Float32Array,
  width: number,
  height: number,
  x: number,
  y: number,
): number {
  const column = Math.floor(x);
  const row = Math.floor(y);
  const tx = x - column;
  const ty = y - row;
  const read = (c: number, r: number): number => {
    if (c < 0 || r < 0 || c >= width || r >= height) return 0;
    return values[r * width + c]!;
  };
  const south = read(column, row) * (1 - tx) + read(column + 1, row) * tx;
  const north = read(column, row + 1) * (1 - tx) + read(column + 1, row + 1) * tx;
  return south * (1 - ty) + north * ty;
}

function clampByte(value: number): number {
  return value < 0 ? 0 : value > 255 ? 255 : Math.round(value);
}

/** A ramp as CSS colours, for the legend. */
export function rampSwatches(ramp: readonly RampStop[], steps: number): string[] {
  const swatches: string[] = [];
  for (let index = 0; index < steps; index += 1) {
    const [red, green, blue] = rampColour(ramp, steps === 1 ? 0 : index / (steps - 1));
    swatches.push(
      `rgb(${Math.round(red)} ${Math.round(green)} ${Math.round(blue)})`,
    );
  }
  return swatches;
}
