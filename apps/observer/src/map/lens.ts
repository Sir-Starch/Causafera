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
  {
    id: "geography",
    title: {
      "en-US": "Geography",
      "ru-RU": "География",
      "zh-Hans": "地理",
      "de-DE": "Geographie",
      "es-ES": "Geografía",
    },
  },
  {
    id: "material",
    title: {
      "en-US": "Material",
      "ru-RU": "Вещество",
      "zh-Hans": "物质",
      "de-DE": "Materie",
      "es-ES": "Materia",
    },
  },
  {
    id: "mana",
    title: {
      "en-US": "Mana",
      "ru-RU": "Мана",
      "zh-Hans": "魔力",
      "de-DE": "Mana",
      "es-ES": "Maná",
    },
  },
  {
    id: "life",
    title: {
      "en-US": "Living systems",
      "ru-RU": "Живое",
      "zh-Hans": "生命系统",
      "de-DE": "Lebende Systeme",
      "es-ES": "Sistemas vivos",
    },
  },
  {
    id: "causality",
    title: {
      "en-US": "Causality",
      "ru-RU": "Причинность",
      "zh-Hans": "因果性",
      "de-DE": "Kausalität",
      "es-ES": "Causalidad",
    },
  },
  {
    id: "cognition",
    title: {
      "en-US": "Cognition",
      "ru-RU": "Познание",
      "zh-Hans": "认知",
      "de-DE": "Kognition",
      "es-ES": "Cognición",
    },
  },
  {
    id: "society",
    title: {
      "en-US": "Society",
      "ru-RU": "Общество",
      "zh-Hans": "社会",
      "de-DE": "Gesellschaft",
      "es-ES": "Sociedad",
    },
  },
];

export const AVAILABILITY_TITLE: Record<LensAvailability, Record<ObserverLocale, string>> = {
  observed: {
    "en-US": "Observed",
    "ru-RU": "Наблюдается",
    "zh-Hans": "已观测",
    "de-DE": "Beobachtet",
    "es-ES": "Observada",
  },
  partial: {
    "en-US": "Partial",
    "ru-RU": "Частично",
    "zh-Hans": "部分",
    "de-DE": "Teilweise",
    "es-ES": "Parcial",
  },
  preview: {
    "en-US": "Preview",
    "ru-RU": "Прототип",
    "zh-Hans": "预览",
    "de-DE": "Vorschau",
    "es-ES": "Vista previa",
  },
  awaiting: {
    "en-US": "Awaiting",
    "ru-RU": "Ожидает",
    "zh-Hans": "等待中",
    "de-DE": "Ausstehend",
    "es-ES": "A la espera",
  },
};

export const AVAILABILITY_DETAIL: Record<LensAvailability, Record<ObserverLocale, string>> = {
  observed: {
    "en-US": "Real observer data through the current protocol.",
    "ru-RU": "Реальные данные наблюдателя через текущий протокол.",
    "zh-Hans": "经由当前协议获得的真实观测器数据。",
    "de-DE": "Echte Beobachterdaten über das aktuelle Protokoll.",
    "es-ES": "Datos reales del observador a través del protocolo actual.",
  },
  partial: {
    "en-US": "Real data, but only a narrow slice of what the lens names.",
    "ru-RU": "Реальные данные, но только узкий срез того, что называет линза.",
    "zh-Hans": "真实数据，但只是该透镜所指称内容的一个窄切片。",
    "de-DE": "Echte Daten, aber nur ein schmaler Ausschnitt dessen, was die Linse benennt.",
    "es-ES": "Datos reales, pero sólo una porción estrecha de lo que la lente nombra.",
  },
  preview: {
    "en-US":
      "An observer-side construction over real values. It is not a measurement: the lens is marked as a preview.",
    "ru-RU":
      "Построение на стороне наблюдателя из реальных величин. Это не измерение: линза помечена как прототип.",
    "zh-Hans": "基于真实数值的观测器端构造。它不是测量：该透镜已标注为预览。",
    "de-DE":
      "Eine beobachterseitige Konstruktion über echten Werten. Sie ist keine Messung: die Linse ist als Vorschau gekennzeichnet.",
    "es-ES":
      "Una construcción del lado del observador sobre valores reales. No es una medición: la lente está marcada como vista previa.",
  },
  awaiting: {
    "en-US": "No read model yet. The lens is listed and states what it is waiting for.",
    "ru-RU": "Модели чтения ещё нет. Линза перечислена и объясняет, чего ждёт.",
    "zh-Hans": "尚无读取模型。该透镜仍被列出，并说明它在等待什么。",
    "de-DE": "Noch kein Lesemodell. Die Linse wird aufgeführt und nennt, worauf sie wartet.",
    "es-ES":
      "Todavía no hay modelo de lectura. La lente aparece en la lista y declara qué está esperando.",
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
