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
const CACHE_CAPACITY = 12;

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
