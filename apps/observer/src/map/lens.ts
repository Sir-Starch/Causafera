/**
 * Analytical lenses.
 *
 * A lens is one class of information projected onto the chart. The renderer knows nothing
 * about mana, population or provenance: it draws fields, symbols, cell marks, vectors and
 * isolines, and a lens decides which of those it produces from the observer payloads it is
 * given. Adding a domain therefore means adding a lens, never editing the renderer.
 *
 * Availability is part of the contract rather than a footnote. The instrument observes very
 * different depths across domains, and a lens states which of the four it is in — so a lens
 * with no read model behind it is listed, explained, and drawn as unsurveyed ground rather
 * than quietly omitted or, worse, invented.
 */

import type { RuntimeSummary } from "@causafera/observer-protocol";

import type { ObserverLocale } from "../observer/format";
import type { Atlas, ChunkRecord, GateRecord, SignalId, SurfaceLadder } from "../observer/models";

export type LensId = string;

export type LensAvailability =
  | "observed" //  real observer data through the current protocol
  | "partial" //   real data, but a narrow slice of what the lens names
  | "preview" //   an observer-side construction, exploratory and marked as such
  | "awaiting"; //  no read model yet; the lens states what it is waiting for

export type LensGroupId =
  | "geography"
  | "material"
  | "mana"
  | "life"
  | "causality"
  | "cognition"
  | "society";

/** Where a lens may be mounted. Most are one or the other; a few are both. */
export type LensRole = "primary" | "overlay";

export interface LensContext {
  atlas: Atlas;
  ladders: readonly SurfaceLadder[];
  gates: readonly GateRecord[];
  summary?: RuntimeSummary;
  locale: ObserverLocale;
}

/** A scalar over chunks, drawn as the base fill of the chart. */
export interface LensField {
  min: number;
  max: number;
  /** Chunk key → raw value. Chunks absent from the map are unsurveyed for this lens. */
  values: Map<string, number>;
  format(value: number): string;
}

/** A mark at a chunk centre: a proportional circle, or a fixed survey glyph. */
export interface LensSymbol {
  chunkKey: string;
  chunkX: number;
  chunkY: number;
  /** 0..1 of the symbol's own scale. Ignored by fixed-size shapes. */
  weight: number;
  value: number;
  label: string;
  shape?: "circle" | "cross" | "ring" | "diamond";
}

/** A mark at a real cell position inside a chunk, drawn only at cell detail. */
export interface LensCellMark {
  chunkKey: string;
  chunkX: number;
  chunkY: number;
  cellX: number;
  cellY: number;
  cellZ: number;
  intensity: number;
  shape: "diamond" | "square" | "cross" | "ring";
  label: string;
}

/** A directed magnitude between two chunk centres. */
export interface LensVector {
  fromX: number;
  fromY: number;
  toX: number;
  toY: number;
  /** 0..1 of the longest vector in the set. */
  weight: number;
  label: string;
}

/** A polyline in chart space, in world units. */
export interface LensIsoline {
  points: [number, number][];
  /** 0..1 within the isoline set, used for line weight. */
  level: number;
  label?: string;
}

export interface LensLayers {
  field?: LensField;
  symbols?: LensSymbol[];
  cells?: LensCellMark[];
  vectors?: LensVector[];
  isolines?: LensIsoline[];
}

export interface Lens {
  id: LensId;
  group: LensGroupId;
  signal: SignalId;
  availability: LensAvailability;
  roles: LensRole[];
  title: Record<ObserverLocale, string>;
  detail: Record<ObserverLocale, string>;
  unit?: Record<ObserverLocale, string>;
  /** For partial, preview and awaiting lenses: what is missing, or what was constructed. */
  caveat?: Record<ObserverLocale, string>;
  /** Whether the lens has anything to say once the cell lattice is visible. */
  cellProjection: "none" | "partial" | "full";
  /** Produces everything the renderer will draw. Returns empty layers when it has no data. */
  layers(context: LensContext): LensLayers;
}

export interface LensGroup {
  id: LensGroupId;
  title: Record<ObserverLocale, string>;
}

export const LENS_GROUPS: LensGroup[] = [
  { id: "geography", title: { "ru-RU": "География", "en-US": "Geography" } },
  { id: "material", title: { "ru-RU": "Вещество", "en-US": "Material" } },
  { id: "mana", title: { "ru-RU": "Мана", "en-US": "Mana" } },
  { id: "life", title: { "ru-RU": "Живое", "en-US": "Living systems" } },
  { id: "causality", title: { "ru-RU": "Причинность", "en-US": "Causality" } },
  { id: "cognition", title: { "ru-RU": "Познание", "en-US": "Cognition" } },
  { id: "society", title: { "ru-RU": "Общество", "en-US": "Society" } },
];

export const AVAILABILITY_TITLE: Record<LensAvailability, Record<ObserverLocale, string>> = {
  observed: { "ru-RU": "Наблюдается", "en-US": "Observed" },
  partial: { "ru-RU": "Частично", "en-US": "Partial" },
  preview: { "ru-RU": "Прототип", "en-US": "Preview" },
  awaiting: { "ru-RU": "Ожидает", "en-US": "Awaiting" },
};

export const AVAILABILITY_DETAIL: Record<LensAvailability, Record<ObserverLocale, string>> = {
  observed: {
    "ru-RU": "Реальные данные наблюдателя через текущий протокол.",
    "en-US": "Real observer data through the current protocol.",
  },
  partial: {
    "ru-RU": "Реальные данные, но только узкий срез того, что называет линза.",
    "en-US": "Real data, but only a narrow slice of what the lens names.",
  },
  preview: {
    "ru-RU":
      "Построение на стороне наблюдателя из реальных величин. Это не измерение: линза помечена как прототип.",
    "en-US":
      "An observer-side construction over real values. It is not a measurement: the lens is marked as a preview.",
  },
  awaiting: {
    "ru-RU": "Модели чтения ещё нет. Линза перечислена и объясняет, чего ждёт.",
    "en-US": "No read model yet. The lens is listed and states what it is waiting for.",
  },
};

/** A lens may be chosen as the base field only when it can actually draw one. */
export function canBePrimary(lens: Lens): boolean {
  return lens.roles.includes("primary");
}

export function isDrawable(lens: Lens): boolean {
  return lens.availability !== "awaiting";
}

export const EMPTY_LAYERS: LensLayers = {};

/** Normalise a chunk-keyed scalar into a field, skipping the degenerate single-value case. */
export function fieldFrom(
  chunks: readonly ChunkRecord[],
  read: (chunk: ChunkRecord) => number | undefined,
  format: (value: number) => string,
): LensField | undefined {
  const values = new Map<string, number>();
  for (const chunk of chunks) {
    const value = read(chunk);
    if (value !== undefined) values.set(chunk.key, value);
  }
  if (values.size === 0) return undefined;
  const all = [...values.values()];
  const min = Math.min(...all);
  const max = Math.max(...all);
  return { min, max: max === min ? min + 1 : max, values, format };
}

/** Position of `value` in a field, clamped — the renderer's only ramp input. */
export function fieldIntensity(field: LensField, value: number): number {
  return Math.max(0, Math.min(1, (value - field.min) / (field.max - field.min || 1)));
}
