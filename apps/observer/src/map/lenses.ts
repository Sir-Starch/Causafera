/**
 * The lens catalogue.
 *
 * One entry per class of information the observer might project onto the chart. Entries are
 * ordered by group; the renderer never reads this file, and this file never reads the
 * renderer. To connect a future domain, replace an `awaiting` entry's `layers` with a real
 * projection and change its availability — nothing else in the map changes.
 *
 * `caveat` on a partial, preview or awaiting lens names the exact read model that is missing;
 * those names correspond to `docs/ui/observer-projection-gaps.md`.
 */

import { formatCompact, formatInteger, formatMillimetresAsMetres } from "../observer/format";
import { CHUNK_SIZE } from "../observer/models";
import {
  EMPTY_LAYERS,
  fieldFrom,
  type Lens,
  type LensCellMark,
  type LensContext,
  type LensLayers,
  type LensSymbol,
} from "./lens";
import { previewGradient, previewIsolines } from "./preview";

/** An entry that exists so the instrument can state its own range honestly. */
function awaiting(
  id: string,
  group: Lens["group"],
  signal: Lens["signal"],
  title: Lens["title"],
  detail: Lens["detail"],
  caveat: Lens["detail"],
): Lens {
  return {
    id,
    group,
    signal,
    availability: "awaiting",
    roles: ["primary"],
    title,
    detail,
    caveat,
    cellProjection: "none",
    layers: (): LensLayers => EMPTY_LAYERS,
  };
}

export const LENSES: Lens[] = [
  /* ---------------------------------------------------------- geography -- */
  {
    id: "relief",
    group: "geography",
    signal: "physical",
    availability: "observed",
    roles: ["primary"],
    title: { "ru-RU": "Рельеф", "en-US": "Relief" },
    detail: {
      "ru-RU": "Наибольшая высота чанка по данным террейн-носителя.",
      "en-US": "The chunk's greatest elevation, from the terrain carrier.",
    },
    unit: { "ru-RU": "м", "en-US": "m" },
    cellProjection: "none",
    caveat: {
      "ru-RU": "Высота по ячейкам наблюдателю не проецируется — на масштабе ячеек линза молчит.",
      "en-US": "Per-cell elevation is not projected, so the lens is silent at cell scale.",
    },
    layers: (context) => ({
      field: fieldFrom(
        context.atlas.chunks,
        (chunk) => chunk.maximumElevationMm,
        (value) => `${formatMillimetresAsMetres(value, 1)} м`,
      ),
    }),
  },
  {
    id: "relief-range",
    group: "geography",
    signal: "physical",
    availability: "observed",
    roles: ["primary"],
    title: { "ru-RU": "Перепад высот", "en-US": "Elevation range" },
    detail: {
      "ru-RU": "Разность наибольшей и наименьшей высоты внутри чанка.",
      "en-US": "The difference between the chunk's greatest and least elevation.",
    },
    unit: { "ru-RU": "м", "en-US": "m" },
    cellProjection: "none",
    layers: (context) => ({
      field: fieldFrom(
        context.atlas.chunks,
        (chunk) => chunk.maximumElevationMm - chunk.minimumElevationMm,
        (value) => `${formatMillimetresAsMetres(value, 1)} м`,
      ),
    }),
  },
  {
    id: "roughness",
    group: "geography",
    signal: "physical",
    availability: "observed",
    roles: ["primary"],
    title: { "ru-RU": "Шероховатость", "en-US": "Roughness" },
    detail: {
      "ru-RU": "Средняя шероховатость поверхности чанка.",
      "en-US": "Mean surface roughness across the chunk.",
    },
    unit: { "ru-RU": "мм", "en-US": "mm" },
    cellProjection: "none",
    layers: (context) => ({
      field: fieldFrom(
        context.atlas.chunks,
        (chunk) => chunk.meanRoughnessMm,
        (value) => `${Math.round(value)} мм`,
      ),
    }),
  },
  {
    id: "contours",
    group: "geography",
    signal: "physical",
    availability: "preview",
    roles: ["overlay"],
    title: { "ru-RU": "Изогипсы", "en-US": "Contours" },
    detail: {
      "ru-RU": "Изолинии по интерполированной высоте чанков.",
      "en-US": "Isolines over interpolated chunk elevation.",
    },
    caveat: {
      "ru-RU":
        "Между центрами чанков высота интерполирована наблюдателем. Это способ читать значения чанков, а не измеренный рельеф.",
      "en-US":
        "Elevation between chunk centres is interpolated by the observer. It is a way of reading chunk values, not measured terrain.",
    },
    cellProjection: "none",
    layers: (context) => ({
      isolines: previewIsolines(
        context.atlas.chunks,
        (chunk) => chunk.maximumElevationMm,
        9,
        (value) => `${formatMillimetresAsMetres(value, 0)} м`,
      ),
    }),
  },
  awaiting(
    "ecology",
    "geography",
    "life",
    { "ru-RU": "Экология", "en-US": "Ecology" },
    {
      "ru-RU": "Биомы, покров, климатические режимы и их границы.",
      "en-US": "Biomes, cover, climate regimes and their boundaries.",
    },
    {
      "ru-RU": "Домен документирован, но не реализован: проецировать нечего.",
      "en-US": "The domain is documented but not implemented: there is nothing to project.",
    },
  ),

  /* ----------------------------------------------------------- material -- */
  {
    id: "surface",
    group: "material",
    signal: "physical",
    availability: "partial",
    roles: ["primary", "overlay"],
    title: { "ru-RU": "Материальная поверхность", "en-US": "Material surface" },
    detail: {
      "ru-RU": "Зафиксированные переходы состояния поверхности в окне наблюдения.",
      "en-US": "Committed surface condition transitions within the observation window.",
    },
    caveat: {
      "ru-RU":
        "Среда отслеживает по одной поверхности на чанк, и окно ограничено 64 переходами. Остальные ячейки не наблюдаются.",
      "en-US":
        "The runtime tracks one surface per chunk and the window is bounded at 64 transitions. Other cells are not observed.",
    },
    cellProjection: "partial",
    layers: (context) => {
      const perChunk = new Map<string, number>();
      for (const ladder of context.ladders) {
        const key = `${ladder.chartId}:${ladder.chunkX}:${ladder.chunkY}:${ladder.chunkZ}`;
        perChunk.set(key, (perChunk.get(key) ?? 0) + ladder.steps.length);
      }
      const cells: LensCellMark[] = context.ladders.map((ladder) => {
        const last = ladder.steps[ladder.steps.length - 1];
        return {
          chunkKey: `${ladder.chartId}:${ladder.chunkX}:${ladder.chunkY}:${ladder.chunkZ}`,
          chunkX: ladder.chunkX,
          chunkY: ladder.chunkY,
          cellX: ladder.cell.x,
          cellY: ladder.cell.y,
          cellZ: ladder.cell.z,
          intensity: ladder.maxCondition <= 0 ? 0 : Math.min(1, ladder.maxCondition / 80),
          shape: "diamond",
          label: `#${ladder.cellOrdinal} · ${last?.afterCondition ?? 0}`,
        };
      });
      const busiest = Math.max(1, ...perChunk.values());
      return {
        field: fieldFrom(
          context.atlas.chunks,
          (chunk) => perChunk.get(chunk.key) ?? 0,
          (value) => `${Math.round(value)}`,
        ),
        // At chunk scale the overlay counts transitions; at cell scale it resolves to the
        // real lattice positions above.
        symbols: context.atlas.chunks
          .filter((chunk) => (perChunk.get(chunk.key) ?? 0) > 0)
          .map((chunk) => ({
            chunkKey: chunk.key,
            chunkX: chunk.chunkX,
            chunkY: chunk.chunkY,
            weight: Math.sqrt((perChunk.get(chunk.key) ?? 0) / busiest),
            value: perChunk.get(chunk.key) ?? 0,
            label: `${perChunk.get(chunk.key) ?? 0}`,
            shape: "diamond" as const,
          })),
        cells,
      };
    },
  },

  /* --------------------------------------------------------------- mana -- */
  {
    id: "mana",
    group: "mana",
    signal: "mana",
    availability: "observed",
    roles: ["primary"],
    title: { "ru-RU": "Поле маны", "en-US": "Mana field" },
    detail: {
      "ru-RU": "Суммарная интенсивность маны в чанке.",
      "en-US": "Total mana intensity within the chunk.",
    },
    cellProjection: "none",
    caveat: {
      "ru-RU":
        "Среда хранит поле по ячейкам, наблюдателю проецируется только сумма по чанку и пиковая ячейка.",
      "en-US":
        "The runtime holds a per-cell field; only the chunk total and the peak cell are projected.",
    },
    layers: (context) => ({
      field: fieldFrom(
        context.atlas.chunks,
        (chunk) => chunk.manaTotal,
        (value) => formatCompact(value, context.locale),
      ),
    }),
  },
  {
    id: "mana-gradient",
    group: "mana",
    signal: "mana",
    availability: "preview",
    roles: ["overlay"],
    title: { "ru-RU": "Градиент маны", "en-US": "Mana gradient" },
    detail: {
      "ru-RU": "Разность маны между соседними чанками, стрелкой в сторону большего значения.",
      "en-US": "The mana difference between neighbouring chunks, arrowed towards the larger value.",
    },
    caveat: {
      "ru-RU":
        "Это разность, а не измеренный поток: члена переноса между чанками наблюдателю не передают.",
      "en-US":
        "This is a difference, not a measured flux: no transport term between chunks is projected.",
    },
    cellProjection: "none",
    layers: (context) => ({
      vectors: previewGradient(
        context.atlas.chunks,
        (chunk) => chunk.manaTotal,
        (value) => `+${formatCompact(value, context.locale)}`,
      ),
    }),
  },
  {
    id: "gates",
    group: "mana",
    signal: "resolution",
    availability: "partial",
    roles: ["overlay"],
    title: { "ru-RU": "Затворы локальной маны", "en-US": "Local mana gates" },
    detail: {
      "ru-RU": "Ячейки, в которых затвор локальной маны закрылся.",
      "en-US": "Cells where a local mana gate closed.",
    },
    caveat: {
      "ru-RU":
        "Проецируются только переходы в закрытое состояние. Пустой слой — результат наблюдения, а не пробел.",
      "en-US":
        "Only transitions into the closed state are projected. An empty layer is an observation, not a gap.",
    },
    cellProjection: "partial",
    layers: (context) => ({
      cells: context.gates.map((gate) => {
        const cell = cellOf(gate.cellOrdinal);
        return {
          chunkKey: `${gate.chartId}:${gate.chunkX}:${gate.chunkY}:${gate.chunkZ}`,
          chunkX: gate.chunkX,
          chunkY: gate.chunkY,
          cellX: cell.x,
          cellY: cell.y,
          cellZ: cell.z,
          intensity: 1,
          shape: "square" as const,
          label: `${gate.localManaBefore} → ${gate.localManaAfter}`,
        };
      }),
    }),
  },

  /* --------------------------------------------------------------- life -- */
  {
    id: "population",
    group: "life",
    signal: "life",
    availability: "observed",
    roles: ["primary", "overlay"],
    title: { "ru-RU": "Население", "en-US": "Population" },
    detail: {
      "ru-RU": "Численность населения, отнесённого к чанку.",
      "en-US": "Population count attributed to the chunk.",
    },
    cellProjection: "none",
    caveat: {
      "ru-RU": "Доступна только сумма по чанку: отдельных особей наблюдателю не проецируют.",
      "en-US": "Only the chunk total is available; individuals are not projected.",
    },
    layers: (context) => {
      const maximum = Math.max(1, ...context.atlas.chunks.map((chunk) => chunk.populationTotal));
      const symbols: LensSymbol[] = context.atlas.chunks
        .filter((chunk) => chunk.populationTotal > 0)
        .map((chunk) => ({
          chunkKey: chunk.key,
          chunkX: chunk.chunkX,
          chunkY: chunk.chunkY,
          // Area-proportional, so a circle twice the area means twice the population.
          weight: Math.sqrt(chunk.populationTotal / maximum),
          value: chunk.populationTotal,
          label: formatInteger(chunk.populationTotal, context.locale),
          shape: "circle" as const,
        }));
      return {
        field: fieldFrom(
          context.atlas.chunks,
          (chunk) => chunk.populationTotal,
          (value) => formatInteger(Math.round(value), context.locale),
        ),
        symbols,
      };
    },
  },

  /* ---------------------------------------------------------- causality -- */
  {
    id: "causal-activity",
    group: "causality",
    signal: "trace",
    availability: "observed",
    roles: ["primary"],
    title: { "ru-RU": "Причинная активность", "en-US": "Causal activity" },
    detail: {
      "ru-RU": "Число причинных событий, отнесённых к чанку.",
      "en-US": "Causal events attributed to the chunk.",
    },
    cellProjection: "none",
    layers: (context) => ({
      field: fieldFrom(
        context.atlas.chunks,
        (chunk) => chunk.causalEventCount,
        (value) => formatInteger(Math.round(value), context.locale),
      ),
    }),
  },
  {
    id: "trace-anchors",
    group: "causality",
    signal: "trace",
    availability: "partial",
    roles: ["overlay"],
    title: { "ru-RU": "Якоря трасс", "en-US": "Trace anchors" },
    detail: {
      "ru-RU": "Последняя трасса, привязанная к каждому чанку.",
      "en-US": "The latest trace anchored to each chunk.",
    },
    caveat: {
      "ru-RU": "Предков трассы запросить нельзя: якорь показывает, что связь есть, но не куда ведёт.",
      "en-US":
        "Trace ancestry cannot be queried: an anchor shows that a link exists, not where it leads.",
    },
    cellProjection: "none",
    layers: (context) => ({
      symbols: context.atlas.chunks.map((chunk) => ({
        chunkKey: chunk.key,
        chunkX: chunk.chunkX,
        chunkY: chunk.chunkY,
        weight: 1,
        value: Number(chunk.latestTraceId),
        label: `#${chunk.latestTraceId}`,
        shape: "cross" as const,
      })),
    }),
  },
  {
    id: "resolution",
    group: "causality",
    signal: "resolution",
    availability: "observed",
    roles: ["primary"],
    title: { "ru-RU": "Причинное разрешение", "en-US": "Causal resolution" },
    detail: {
      "ru-RU": "Релевантность чанка и достигнутый уровень детализации.",
      "en-US": "Chunk relevance and the level of detail it has reached.",
    },
    cellProjection: "none",
    caveat: {
      "ru-RU":
        "Пороги переключения уровней политикой не проецируются, поэтому шкала показана в сырых единицах.",
      "en-US":
        "The policy thresholds between levels are not projected, so the scale is shown in raw units.",
    },
    layers: (context) => ({
      field: fieldFrom(
        context.atlas.chunks,
        (chunk) => chunk.resolutionRelevance,
        (value) => formatInteger(Math.round(value), context.locale),
      ),
      symbols: context.atlas.chunks.map((chunk) => ({
        chunkKey: chunk.key,
        chunkX: chunk.chunkX,
        chunkY: chunk.chunkY,
        weight: 1,
        value: chunk.resolutionLevel,
        label: `L${chunk.resolutionLevel}`,
        shape: "ring" as const,
      })),
    }),
  },
  awaiting(
    "provenance",
    "causality",
    "trace",
    { "ru-RU": "Граф происхождения", "en-US": "Provenance graph" },
    {
      "ru-RU": "Цепочки предков между зафиксированными событиями, наложенные на пространство.",
      "en-US": "Ancestry chains between committed events, laid over space.",
    },
    {
      "ru-RU": "Нужен вид запроса к CausalTraceStore с ограниченным окном предков.",
      "en-US": "Needs a bounded ancestry query kind over CausalTraceStore.",
    },
  ),

  /* ---------------------------------------------------------- cognition -- */
  awaiting(
    "agents",
    "cognition",
    "life",
    { "ru-RU": "Агенты", "en-US": "Agents" },
    {
      "ru-RU": "Положение, состояние и действия отдельных акторов.",
      "en-US": "Position, state and actions of individual actors.",
    },
    {
      "ru-RU": "Схема EntitySummary описана в протоколе, модели чтения нет.",
      "en-US": "The EntitySummary schema exists in the protocol; no read model does.",
    },
  ),
  awaiting(
    "knowledge",
    "cognition",
    "resolution",
    { "ru-RU": "Знание и убеждения", "en-US": "Knowledge and belief" },
    {
      "ru-RU":
        "Что агенты считают известным о местности — субъективная карта поверх объективной.",
      "en-US": "What agents hold as known about the ground — a subjective chart over the objective one.",
    },
    {
      "ru-RU":
        "Требует моделей чтения субъективной сцены и убеждений; когниция на уровне контрактов.",
      "en-US":
        "Requires subjective scene and belief read models; cognition is at contract level.",
    },
  ),
  awaiting(
    "language",
    "cognition",
    "mana",
    { "ru-RU": "Язык", "en-US": "Language" },
    {
      "ru-RU": "Распространение лексем и семантический дрейф по территории.",
      "en-US": "Lexeme spread and semantic drift across the territory.",
    },
    {
      "ru-RU": "Языковой домен не связан со средой; проецировать нечего.",
      "en-US": "The language domain is not coupled to the runtime; there is nothing to project.",
    },
  ),

  /* ------------------------------------------------------------ society -- */
  awaiting(
    "social",
    "society",
    "life",
    { "ru-RU": "Социальная структура", "en-US": "Social structure" },
    {
      "ru-RU": "Связи, группы и институты, размещённые в пространстве.",
      "en-US": "Ties, groups and institutions placed in space.",
    },
    {
      "ru-RU": "Нужны выводимая агентами структура и модель чтения.",
      "en-US": "Needs agent-inferred structure and a read model.",
    },
  ),
  awaiting(
    "practices",
    "society",
    "physical",
    { "ru-RU": "Практики", "en-US": "Practices" },
    {
      "ru-RU": "Передача и мутации практик между местами.",
      "en-US": "Transmission and mutation of practices between places.",
    },
    {
      "ru-RU": "Требуется воплощённое исполнение практик, которого пока нет.",
      "en-US": "Requires embodied practice execution, which does not exist yet.",
    },
  ),
  awaiting(
    "economy",
    "society",
    "mana",
    { "ru-RU": "Хозяйство", "en-US": "Economy" },
    {
      "ru-RU": "Потоки материала, запасы и обмен между местами.",
      "en-US": "Material flows, stocks and exchange between places.",
    },
    {
      "ru-RU": "Городской и материальный домены не имеют модели чтения для наблюдателя.",
      "en-US": "The city and material domains have no observer read model.",
    },
  ),
];

export const LENS_BY_ID = new Map(LENSES.map((lens) => [lens.id, lens]));

export const DEFAULT_PRIMARY_LENS = "relief";
export const DEFAULT_OVERLAYS = ["population", "surface"];

/** The lattice ordinal decoded back to a cell position. */
function cellOf(ordinal: number): { x: number; y: number; z: number } {
  return {
    x: ordinal % CHUNK_SIZE,
    y: Math.floor(ordinal / CHUNK_SIZE) % CHUNK_SIZE,
    z: Math.floor(ordinal / (CHUNK_SIZE * CHUNK_SIZE)),
  };
}

/** Resolve stored identifiers back to lenses, dropping anything the catalogue no longer has. */
export function resolveLenses(ids: readonly string[]): Lens[] {
  return ids
    .map((id) => LENS_BY_ID.get(id))
    .filter((lens): lens is Lens => lens !== undefined && lens.availability !== "awaiting");
}

export function lensLayers(lens: Lens, context: LensContext): LensLayers {
  try {
    return lens.layers(context);
  } catch {
    // A lens must never take the map down with it.
    return EMPTY_LAYERS;
  }
}
