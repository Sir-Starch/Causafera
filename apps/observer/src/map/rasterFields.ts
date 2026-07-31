/**
 * From received lattices to drawable fields.
 *
 * The observer receives one raster per chunk per field. Everything the map draws
 * as a continuous surface passes through here, where the per-chunk lattices are
 * reduced to plan view if they are volumetric and then assembled into one field
 * over the whole surveyed extent.
 *
 * The plan-view reduction of a volumetric field lives on this side of the wire
 * on purpose. A column sum answers "how much stands over this ground" and a
 * column maximum answers "how intense does it get anywhere in this column";
 * choosing between them is a reading of the field rather than a property of it,
 * so the runtime projects the volume unreduced and the map states which reading
 * it is showing.
 */

import { FieldRasterKind, type FieldRaster } from "@causafera/observer-protocol";

import { assembleField, type ChartField, type FieldPatch } from "./field";

export type ColumnReading = "sum" | "maximum";

/** Which band of a raster a field is built from. */
export type RasterBand = "value" | "auxiliary";

const cache = new Map<string, ChartField | undefined>();
/**
 * Assembled fields held at once.
 *
 * A frame can want seven at a time — terrain's two bands, the mana volume's two
 * readings, and the three water buckets — and four of those change every tick.
 * The capacity has to clear the live set plus one frame of churn, or the two
 * fields that never change get evicted by the ones that always do and are
 * rebuilt from scratch on every frame.
 */
const CACHE_CAPACITY = 24;

/**
 * Assembled fields are memoised against their own signature.
 *
 * Every lens mounted on the chart asks for its field on every render, and the
 * signature already identifies the measurements exactly — a field rebuilt from
 * the same rasters is the same field.
 */
function memoise(signature: string, build: () => ChartField | undefined): ChartField | undefined {
  if (cache.has(signature)) return cache.get(signature);
  const field = build();
  while (cache.size >= CACHE_CAPACITY) {
    const oldest = cache.keys().next().value;
    if (oldest === undefined) break;
    cache.delete(oldest);
  }
  cache.set(signature, field);
  return field;
}

function rastersOfKind(
  rasters: ReadonlyMap<string, FieldRaster>,
  kind: FieldRasterKind,
): FieldRaster[] {
  const found: FieldRaster[] = [];
  for (const [key, raster] of rasters) {
    if (key.startsWith(`${kind}|`)) found.push(raster);
  }
  return found.sort((a, b) => a.chunkY - b.chunkY || a.chunkX - b.chunkX);
}

/** Identity of a set of rasters: the provenance each one arrived with. */
function signatureOf(rasters: readonly FieldRaster[], suffix: string): string {
  return `${suffix}:${rasters
    .map((raster) => `${raster.chunkX},${raster.chunkY}=${raster.generationTraceId}`)
    .join(";")}`;
}

/** A surface field: one sample per cell, taken from the named band. */
export function surfaceField(
  rasters: ReadonlyMap<string, FieldRaster>,
  kind: FieldRasterKind,
  band: RasterBand = "value",
): ChartField | undefined {
  const found = rastersOfKind(rasters, kind);
  if (found.length === 0) return undefined;
  return memoise(signatureOf(found, `${kind}/${band}`), () => {
    const patches: FieldPatch[] = [];
    for (const raster of found) {
      const values = band === "auxiliary" ? raster.auxiliary : raster.values;
      if (values.length !== raster.edge * raster.edge * raster.depth) continue;
      patches.push({
        chunkX: raster.chunkX,
        chunkY: raster.chunkY,
        edge: raster.edge,
        values: raster.depth === 1 ? values : columnReduce(values, raster, "sum"),
      });
    }
    return assembleField(patches);
  });
}

/**
 * A surface field built from the lossless unsigned band a hydrology lattice
 * carries.
 *
 * The band is `BigUint64Array` because a water volume is a `u64` and the upper
 * half of that range has no image in a double. Painting a surface needs
 * doubles, so the conversion happens here and nowhere earlier: the drawn field
 * is a picture of the measurements, while the exact counts stay in the raster
 * for any surface that reports a number rather than a colour. The lenses that
 * use this say so in their caveat.
 */
export function unsignedSurfaceField(
  rasters: ReadonlyMap<string, FieldRaster>,
  kind: FieldRasterKind,
): ChartField | undefined {
  const found = rastersOfKind(rasters, kind);
  if (found.length === 0) return undefined;
  return memoise(signatureOf(found, `${kind}/unsigned`), () => {
    const patches: FieldPatch[] = [];
    for (const raster of found) {
      if (raster.unsignedValues.length !== raster.edge * raster.edge * raster.depth) continue;
      const values = new Float64Array(raster.unsignedValues.length);
      for (let index = 0; index < values.length; index += 1) {
        values[index] = Number(raster.unsignedValues[index]!);
      }
      patches.push({
        chunkX: raster.chunkX,
        chunkY: raster.chunkY,
        edge: raster.edge,
        values,
      });
    }
    return assembleField(patches);
  });
}

/**
 * The provenance a set of received lattices arrived with, as one string.
 *
 * A lens signature has to identify the measurements exactly, and for a field
 * that changes every tick the extremes do not: water moving between two cells
 * that both stay inside the current range leaves min, max and the patch count
 * untouched, and a signature built from those alone would hold a stale image on
 * the chart. The generation trace is what actually changed.
 */
export function rasterGeneration(
  rasters: ReadonlyMap<string, FieldRaster>,
  kind: FieldRasterKind,
): string {
  return rastersOfKind(rasters, kind)
    .map((raster) => raster.generationTraceId)
    .join(",");
}

/**
 * The greatest volume any cell of a received hydrology lattice holds.
 *
 * Read from the unsigned band rather than from the assembled field, so the
 * figure a legend states is an exact count and not the double the surface was
 * painted from.
 */
export function unsignedPeak(
  rasters: ReadonlyMap<string, FieldRaster>,
  kind: FieldRasterKind,
): bigint | undefined {
  const found = rastersOfKind(rasters, kind);
  if (found.length === 0) return undefined;
  let peak = 0n;
  for (const raster of found) {
    for (const value of raster.unsignedValues) if (value > peak) peak = value;
  }
  return peak;
}

/**
 * One chunk's exact total for an unsigned lattice, or undefined when none is
 * held for it.
 *
 * Summed in `bigint` so the figure a panel prints is the same count the runtime
 * conserves, rather than the double the surface was painted from.
 */
export function unsignedChunkTotal(
  rasters: ReadonlyMap<string, FieldRaster>,
  kind: FieldRasterKind,
  chunk: { chartId: bigint; chunkX: number; chunkY: number; chunkZ: number },
): bigint | undefined {
  const raster = rasters.get(
    `${kind}|${chunk.chartId}:${chunk.chunkX}:${chunk.chunkY}:${chunk.chunkZ}`,
  );
  if (raster === undefined || raster.unsignedValues.length === 0) return undefined;
  let total = 0n;
  for (const value of raster.unsignedValues) total += value;
  return total;
}

/** A volumetric field reduced through z to the plan view the chart draws. */
export function columnField(
  rasters: ReadonlyMap<string, FieldRaster>,
  kind: FieldRasterKind,
  reading: ColumnReading,
): ChartField | undefined {
  const found = rastersOfKind(rasters, kind);
  if (found.length === 0) return undefined;
  return memoise(signatureOf(found, `${kind}/column-${reading}`), () =>
    assembleField(
      found.map((raster) => ({
        chunkX: raster.chunkX,
        chunkY: raster.chunkY,
        edge: raster.edge,
        values: columnReduce(raster.values, raster, reading),
      })),
    ),
  );
}

function columnReduce(
  values: Float64Array,
  raster: Pick<FieldRaster, "edge" | "depth">,
  reading: ColumnReading,
): Float64Array {
  const { edge, depth } = raster;
  const plan = new Float64Array(edge * edge);
  for (let index = 0; index < plan.length; index += 1) {
    let total = 0;
    let peak = Number.NEGATIVE_INFINITY;
    for (let layer = 0; layer < depth; layer += 1) {
      const value = values[layer * edge * edge + index] ?? 0;
      total += value;
      if (value > peak) peak = value;
    }
    plan[index] = reading === "sum" ? total : peak === Number.NEGATIVE_INFINITY ? 0 : peak;
  }
  return plan;
}

/** The finest lattice edge received for a field, or 0 when none was. */
export function receivedEdge(
  rasters: ReadonlyMap<string, FieldRaster>,
  kind: FieldRasterKind,
): number {
  let edge = 0;
  for (const raster of rastersOfKind(rasters, kind)) {
    if (raster.edge > edge) edge = raster.edge;
  }
  return edge;
}

/**
 * Cells whose last change is one named trace, as sample positions inside their
 * own chunk. This is spatial provenance: no other field the observer projects
 * can say which ground a committed event touched.
 */
export function cellsChangedBy(
  rasters: ReadonlyMap<string, FieldRaster>,
  kind: FieldRasterKind,
  trace: bigint,
): { chunkX: number; chunkY: number; edge: number; x: number; y: number; z: number }[] {
  const found: { chunkX: number; chunkY: number; edge: number; x: number; y: number; z: number }[] =
    [];
  for (const raster of rastersOfKind(rasters, kind)) {
    for (let index = 0; index < raster.cellTraces.length; index += 1) {
      if (raster.cellTraces[index] !== trace) continue;
      const edge = raster.edge;
      found.push({
        chunkX: raster.chunkX,
        chunkY: raster.chunkY,
        edge,
        x: index % edge,
        y: Math.floor(index / edge) % edge,
        z: Math.floor(index / (edge * edge)),
      });
    }
  }
  return found;
}
