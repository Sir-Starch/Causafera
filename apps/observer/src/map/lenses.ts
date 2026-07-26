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

import { copyFor } from "../i18n";
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

/**
 * Units are copy, not data. They come from the active dictionary so a metre reads as `m`,
 * `м` or `米` rather than leaking one language into every other.
 */
const METRES: Lens["unit"] = {
  "en-US": "m",
  "ru-RU": "м",
  "zh-Hans": "米",
  "de-DE": "m",
  "es-ES": "m",
};

const MILLIMETRES: Lens["unit"] = {
  "en-US": "mm",
  "ru-RU": "мм",
  "zh-Hans": "毫米",
  "de-DE": "mm",
  "es-ES": "mm",
};

function metreMark(context: LensContext): string {
  return copyFor(context.locale).chart.metres;
}

function millimetreMark(context: LensContext): string {
  return copyFor(context.locale).chart.millimetres;
}

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
    title: {
      "en-US": "Relief",
      "ru-RU": "Рельеф",
      "zh-Hans": "地势",
      "de-DE": "Relief",
      "es-ES": "Relieve",
    },
    detail: {
      "en-US": "The chunk's greatest elevation, from the terrain carrier.",
      "ru-RU": "Наибольшая высота чанка по данным террейн-носителя.",
      "zh-Hans": "该区块的最大高程，来自地形载体。",
      "de-DE": "Die größte Höhe des Chunks, vom Geländeträger.",
      "es-ES": "La mayor elevación del bloque, según el portador de terreno.",
    },
    unit: METRES,
    cellProjection: "none",
    caveat: {
      "en-US": "Per-cell elevation is not projected, so the lens is silent at cell scale.",
      "ru-RU": "Высота по ячейкам наблюдателю не проецируется — на масштабе ячеек линза молчит.",
      "zh-Hans": "逐单元格的高程不会被投影，因此该透镜在单元格尺度上保持沉默。",
      "de-DE":
        "Die Höhe je Zelle wird nicht projiziert, daher schweigt die Linse auf Zellebene.",
      "es-ES":
        "La elevación por celda no se proyecta, así que la lente calla a escala de celda.",
    },
    layers: (context) => ({
      field: fieldFrom(
        context.atlas.chunks,
        (chunk) => chunk.maximumElevationMm,
        (value) => `${formatMillimetresAsMetres(value, 1)} ${metreMark(context)}`,
      ),
    }),
  },
  {
    id: "relief-range",
    group: "geography",
    signal: "physical",
    availability: "observed",
    roles: ["primary"],
    title: {
      "en-US": "Elevation range",
      "ru-RU": "Перепад высот",
      "zh-Hans": "高程差",
      "de-DE": "Höhenspanne",
      "es-ES": "Rango de elevación",
    },
    detail: {
      "en-US": "The difference between the chunk's greatest and least elevation.",
      "ru-RU": "Разность наибольшей и наименьшей высоты внутри чанка.",
      "zh-Hans": "该区块内最大高程与最小高程之差。",
      "de-DE": "Die Differenz zwischen größter und kleinster Höhe im Chunk.",
      "es-ES": "La diferencia entre la mayor y la menor elevación dentro del bloque.",
    },
    unit: METRES,
    cellProjection: "none",
    layers: (context) => ({
      field: fieldFrom(
        context.atlas.chunks,
        (chunk) => chunk.maximumElevationMm - chunk.minimumElevationMm,
        (value) => `${formatMillimetresAsMetres(value, 1)} ${metreMark(context)}`,
      ),
    }),
  },
  {
    id: "roughness",
    group: "geography",
    signal: "physical",
    availability: "observed",
    roles: ["primary"],
    title: {
      "en-US": "Roughness",
      "ru-RU": "Шероховатость",
      "zh-Hans": "粗糙度",
      "de-DE": "Rauheit",
      "es-ES": "Rugosidad",
    },
    detail: {
      "en-US": "Mean surface roughness across the chunk.",
      "ru-RU": "Средняя шероховатость поверхности чанка.",
      "zh-Hans": "该区块表面的平均粗糙度。",
      "de-DE": "Mittlere Oberflächenrauheit über den Chunk.",
      "es-ES": "Rugosidad media de la superficie a lo largo del bloque.",
    },
    unit: MILLIMETRES,
    cellProjection: "none",
    layers: (context) => ({
      field: fieldFrom(
        context.atlas.chunks,
        (chunk) => chunk.meanRoughnessMm,
        (value) => `${Math.round(value)} ${millimetreMark(context)}`,
      ),
    }),
  },
  {
    id: "contours",
    group: "geography",
    signal: "physical",
    availability: "preview",
    roles: ["overlay"],
    title: {
      "en-US": "Contours",
      "ru-RU": "Изогипсы",
      "zh-Hans": "等高线",
      "de-DE": "Höhenlinien",
      "es-ES": "Curvas de nivel",
    },
    detail: {
      "en-US": "Isolines over interpolated chunk elevation.",
      "ru-RU": "Изолинии по интерполированной высоте чанков.",
      "zh-Hans": "基于插值后区块高程的等值线。",
      "de-DE": "Isolinien über interpolierter Chunk-Höhe.",
      "es-ES": "Isolíneas sobre la elevación interpolada de los bloques.",
    },
    caveat: {
      "en-US":
        "Elevation between chunk centres is interpolated by the observer. It is a way of reading chunk values, not measured terrain.",
      "ru-RU":
        "Между центрами чанков высота интерполирована наблюдателем. Это способ читать значения чанков, а не измеренный рельеф.",
      "zh-Hans":
        "区块中心之间的高程由观测器插值得出。这是一种读取区块数值的方式，而不是实测地形。",
      "de-DE":
        "Die Höhe zwischen den Chunk-Mittelpunkten interpoliert der Beobachter. Das ist eine Lesart der Chunk-Werte, kein gemessenes Gelände.",
      "es-ES":
        "La elevación entre los centros de los bloques la interpola el observador. Es una forma de leer los valores del bloque, no terreno medido.",
    },
    cellProjection: "none",
    layers: (context) => ({
      isolines: previewIsolines(
        context.atlas.chunks,
        (chunk) => chunk.maximumElevationMm,
        9,
        (value) => `${formatMillimetresAsMetres(value, 0)} ${metreMark(context)}`,
      ),
    }),
  },
  awaiting(
    "ecology",
    "geography",
    "life",
    {
      "en-US": "Ecology",
      "ru-RU": "Экология",
      "zh-Hans": "生态",
      "de-DE": "Ökologie",
      "es-ES": "Ecología",
    },
    {
      "en-US": "Biomes, cover, climate regimes and their boundaries.",
      "ru-RU": "Биомы, покров, климатические режимы и их границы.",
      "zh-Hans": "生物群系、地表覆盖、气候型及其边界。",
      "de-DE": "Biome, Bedeckung, Klimaregime und ihre Grenzen.",
      "es-ES": "Biomas, cubierta, regímenes climáticos y sus límites.",
    },
    {
      "en-US": "The domain is documented but not implemented: there is nothing to project.",
      "ru-RU": "Домен документирован, но не реализован: проецировать нечего.",
      "zh-Hans": "该领域已有文档但尚未实现：无可投影之物。",
      "de-DE": "Die Domäne ist dokumentiert, aber nicht implementiert: es gibt nichts zu projizieren.",
      "es-ES": "El dominio está documentado pero no implementado: no hay nada que proyectar.",
    },
  ),

  /* ----------------------------------------------------------- material -- */
  {
    id: "surface",
    group: "material",
    signal: "physical",
    availability: "partial",
    roles: ["primary", "overlay"],
    title: {
      "en-US": "Material surface",
      "ru-RU": "Материальная поверхность",
      "zh-Hans": "物质表面",
      "de-DE": "Materielle Oberfläche",
      "es-ES": "Superficie material",
    },
    detail: {
      "en-US": "Committed surface condition transitions within the observation window.",
      "ru-RU": "Зафиксированные переходы состояния поверхности в окне наблюдения.",
      "zh-Hans": "观测窗口内已提交的表面状态转变。",
      "de-DE": "Festgeschriebene Übergänge des Oberflächenzustands innerhalb des Beobachtungsfensters.",
      "es-ES":
        "Transiciones confirmadas del estado de la superficie dentro de la ventana de observación.",
    },
    caveat: {
      "en-US":
        "The runtime tracks one surface per chunk and the window is bounded at 64 transitions. Other cells are not observed.",
      "ru-RU":
        "Среда отслеживает по одной поверхности на чанк, и окно ограничено 64 переходами. Остальные ячейки не наблюдаются.",
      "zh-Hans": "运行时每个区块只跟踪一个表面，窗口上限为 64 次转变。其余单元格不被观测。",
      "de-DE":
        "Die Laufzeitumgebung verfolgt eine Oberfläche je Chunk, und das Fenster ist auf 64 Übergänge begrenzt. Andere Zellen werden nicht beobachtet.",
      "es-ES":
        "El entorno de ejecución rastrea una superficie por bloque y la ventana está acotada a 64 transiciones. Las demás celdas no se observan.",
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
    title: {
      "en-US": "Mana field",
      "ru-RU": "Поле маны",
      "zh-Hans": "魔力场",
      "de-DE": "Mana-Feld",
      "es-ES": "Campo de maná",
    },
    detail: {
      "en-US": "Total mana intensity within the chunk.",
      "ru-RU": "Суммарная интенсивность маны в чанке.",
      "zh-Hans": "该区块内的魔力总强度。",
      "de-DE": "Die Gesamtintensität des Mana innerhalb des Chunks.",
      "es-ES": "Intensidad total de maná dentro del bloque.",
    },
    cellProjection: "none",
    caveat: {
      "en-US":
        "The runtime holds a per-cell field; only the chunk total and the peak cell are projected.",
      "ru-RU":
        "Среда хранит поле по ячейкам, наблюдателю проецируется только сумма по чанку и пиковая ячейка.",
      "zh-Hans": "运行时持有逐单元格的场；只有区块总量与峰值单元格会被投影。",
      "de-DE":
        "Die Laufzeitumgebung hält ein Feld je Zelle; projiziert werden nur die Chunk-Summe und die Spitzenzelle.",
      "es-ES":
        "El entorno de ejecución mantiene un campo por celda; sólo se proyectan el total del bloque y la celda máxima.",
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
    title: {
      "en-US": "Mana gradient",
      "ru-RU": "Градиент маны",
      "zh-Hans": "魔力梯度",
      "de-DE": "Mana-Gradient",
      "es-ES": "Gradiente de maná",
    },
    detail: {
      "en-US": "The mana difference between neighbouring chunks, arrowed towards the larger value.",
      "ru-RU": "Разность маны между соседними чанками, стрелкой в сторону большего значения.",
      "zh-Hans": "相邻区块之间的魔力差值，箭头指向较大的一侧。",
      "de-DE":
        "Die Mana-Differenz zwischen benachbarten Chunks, mit dem Pfeil zum größeren Wert.",
      "es-ES":
        "La diferencia de maná entre bloques vecinos, con la flecha hacia el valor mayor.",
    },
    caveat: {
      "en-US":
        "This is a difference, not a measured flux: no transport term between chunks is projected.",
      "ru-RU":
        "Это разность, а не измеренный поток: члена переноса между чанками наблюдателю не передают.",
      "zh-Hans": "这是差值，而不是实测通量：区块之间的输运项并不会被投影。",
      "de-DE":
        "Das ist eine Differenz, kein gemessener Fluss: ein Transportterm zwischen Chunks wird nicht projiziert.",
      "es-ES":
        "Esto es una diferencia, no un flujo medido: no se proyecta ningún término de transporte entre bloques.",
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
    title: {
      "en-US": "Local mana gates",
      "ru-RU": "Затворы локальной маны",
      "zh-Hans": "局部魔力闸门",
      "de-DE": "Lokale Mana-Schleusen",
      "es-ES": "Compuertas de maná local",
    },
    detail: {
      "en-US": "Cells where a local mana gate closed.",
      "ru-RU": "Ячейки, в которых затвор локальной маны закрылся.",
      "zh-Hans": "局部魔力闸门发生关闭的单元格。",
      "de-DE": "Zellen, in denen sich eine lokale Mana-Schleuse geschlossen hat.",
      "es-ES": "Celdas donde se cerró una compuerta de maná local.",
    },
    caveat: {
      "en-US":
        "Only transitions into the closed state are projected. An empty layer is an observation, not a gap.",
      "ru-RU":
        "Проецируются только переходы в закрытое состояние. Пустой слой — результат наблюдения, а не пробел.",
      "zh-Hans": "只有转入关闭状态的转变会被投影。空图层是一项观测结果，而不是缺口。",
      "de-DE":
        "Nur Übergänge in den geschlossenen Zustand werden projiziert. Eine leere Ebene ist eine Beobachtung, keine Lücke.",
      "es-ES":
        "Sólo se proyectan las transiciones hacia el estado cerrado. Una capa vacía es una observación, no un hueco.",
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
    title: {
      "en-US": "Population",
      "ru-RU": "Население",
      "zh-Hans": "人口",
      "de-DE": "Bevölkerung",
      "es-ES": "Población",
    },
    detail: {
      "en-US": "Population count attributed to the chunk.",
      "ru-RU": "Численность населения, отнесённого к чанку.",
      "zh-Hans": "归属于该区块的人口数量。",
      "de-DE": "Die dem Chunk zugerechnete Bevölkerungszahl.",
      "es-ES": "Recuento de población atribuido al bloque.",
    },
    cellProjection: "none",
    caveat: {
      "en-US": "Only the chunk total is available; individuals are not projected.",
      "ru-RU": "Доступна только сумма по чанку: отдельных особей наблюдателю не проецируют.",
      "zh-Hans": "只有区块总量可用；个体不会被投影。",
      "de-DE": "Nur die Chunk-Summe ist verfügbar; Einzelwesen werden nicht projiziert.",
      "es-ES": "Sólo está disponible el total del bloque; los individuos no se proyectan.",
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
    title: {
      "en-US": "Causal activity",
      "ru-RU": "Причинная активность",
      "zh-Hans": "因果活动",
      "de-DE": "Kausale Aktivität",
      "es-ES": "Actividad causal",
    },
    detail: {
      "en-US": "Causal events attributed to the chunk.",
      "ru-RU": "Число причинных событий, отнесённых к чанку.",
      "zh-Hans": "归属于该区块的因果事件。",
      "de-DE": "Dem Chunk zugerechnete kausale Ereignisse.",
      "es-ES": "Sucesos causales atribuidos al bloque.",
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
    title: {
      "en-US": "Trace anchors",
      "ru-RU": "Якоря трасс",
      "zh-Hans": "迹线锚点",
      "de-DE": "Spuranker",
      "es-ES": "Anclas de traza",
    },
    detail: {
      "en-US": "The latest trace anchored to each chunk.",
      "ru-RU": "Последняя трасса, привязанная к каждому чанку.",
      "zh-Hans": "锚定到每个区块的最新迹线。",
      "de-DE": "Die neueste an jedem Chunk verankerte Spur.",
      "es-ES": "La traza más reciente anclada a cada bloque.",
    },
    caveat: {
      "en-US":
        "Trace ancestry cannot be queried: an anchor shows that a link exists, not where it leads.",
      "ru-RU": "Предков трассы запросить нельзя: якорь показывает, что связь есть, но не куда ведёт.",
      "zh-Hans": "无法查询迹线的祖先：锚点只说明存在联系，而不说明它通向何处。",
      "de-DE":
        "Die Ahnenkette einer Spur ist nicht abfragbar: ein Anker zeigt, dass eine Verbindung besteht, nicht wohin sie führt.",
      "es-ES":
        "No se puede consultar la ascendencia de una traza: un ancla muestra que existe un vínculo, no adónde lleva.",
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
    title: {
      "en-US": "Causal resolution",
      "ru-RU": "Причинное разрешение",
      "zh-Hans": "因果分辨率",
      "de-DE": "Kausale Auflösung",
      "es-ES": "Resolución causal",
    },
    detail: {
      "en-US": "Chunk relevance and the level of detail it has reached.",
      "ru-RU": "Релевантность чанка и достигнутый уровень детализации.",
      "zh-Hans": "区块的相关度及其已达到的细节层级。",
      "de-DE": "Die Relevanz des Chunks und die erreichte Detailstufe.",
      "es-ES": "La relevancia del bloque y el nivel de detalle que ha alcanzado.",
    },
    cellProjection: "none",
    caveat: {
      "en-US":
        "The policy thresholds between levels are not projected, so the scale is shown in raw units.",
      "ru-RU":
        "Пороги переключения уровней политикой не проецируются, поэтому шкала показана в сырых единицах.",
      "zh-Hans": "各层级之间的策略阈值不会被投影，因此刻度以原始单位显示。",
      "de-DE":
        "Die Richtlinienschwellen zwischen den Stufen werden nicht projiziert, daher erscheint die Skala in Rohwerten.",
      "es-ES":
        "Los umbrales de política entre niveles no se proyectan, así que la escala se muestra en unidades brutas.",
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
    {
      "en-US": "Provenance graph",
      "ru-RU": "Граф происхождения",
      "zh-Hans": "溯源图",
      "de-DE": "Provenienzgraph",
      "es-ES": "Grafo de procedencia",
    },
    {
      "en-US": "Ancestry chains between committed events, laid over space.",
      "ru-RU": "Цепочки предков между зафиксированными событиями, наложенные на пространство.",
      "zh-Hans": "已提交事件之间的祖先链，铺叠在空间之上。",
      "de-DE": "Ahnenketten zwischen festgeschriebenen Ereignissen, über den Raum gelegt.",
      "es-ES": "Cadenas de ascendencia entre sucesos confirmados, tendidas sobre el espacio.",
    },
    {
      "en-US": "Needs a bounded ancestry query kind over CausalTraceStore.",
      "ru-RU": "Нужен вид запроса к CausalTraceStore с ограниченным окном предков.",
      "zh-Hans": "需要针对 CausalTraceStore 的有界祖先查询类型。",
      "de-DE": "Benötigt eine begrenzte Ahnen-Abfrageart über CausalTraceStore.",
      "es-ES": "Necesita un tipo de consulta de ascendencia acotada sobre CausalTraceStore.",
    },
  ),

  /* ---------------------------------------------------------- cognition -- */
  awaiting(
    "agents",
    "cognition",
    "life",
    {
      "en-US": "Agents",
      "ru-RU": "Агенты",
      "zh-Hans": "智能体",
      "de-DE": "Agenten",
      "es-ES": "Agentes",
    },
    {
      "en-US": "Position, state and actions of individual actors.",
      "ru-RU": "Положение, состояние и действия отдельных акторов.",
      "zh-Hans": "单个行动者的位置、状态与动作。",
      "de-DE": "Position, Zustand und Handlungen einzelner Akteure.",
      "es-ES": "Posición, estado y acciones de actores individuales.",
    },
    {
      "en-US": "The EntitySummary schema exists in the protocol; no read model does.",
      "ru-RU": "Схема EntitySummary описана в протоколе, модели чтения нет.",
      "zh-Hans": "协议中已有 EntitySummary 模式，但没有对应的读取模型。",
      "de-DE": "Das EntitySummary-Schema existiert im Protokoll; ein Lesemodell nicht.",
      "es-ES": "El esquema EntitySummary existe en el protocolo; el modelo de lectura no.",
    },
  ),
  awaiting(
    "knowledge",
    "cognition",
    "resolution",
    {
      "en-US": "Knowledge and belief",
      "ru-RU": "Знание и убеждения",
      "zh-Hans": "知识与信念",
      "de-DE": "Wissen und Überzeugung",
      "es-ES": "Conocimiento y creencia",
    },
    {
      "en-US":
        "What agents hold as known about the ground — a subjective chart over the objective one.",
      "ru-RU":
        "Что агенты считают известным о местности — субъективная карта поверх объективной.",
      "zh-Hans": "智能体自认为已知的地面情况——一张覆盖在客观图幅之上的主观图幅。",
      "de-DE":
        "Was Agenten über den Boden für gewusst halten — ein subjektives Kartenblatt über dem objektiven.",
      "es-ES":
        "Lo que los agentes tienen por sabido sobre el terreno: una carta subjetiva sobre la objetiva.",
    },
    {
      "en-US": "Requires subjective scene and belief read models; cognition is at contract level.",
      "ru-RU":
        "Требует моделей чтения субъективной сцены и убеждений; когниция на уровне контрактов.",
      "zh-Hans": "需要主观场景与信念的读取模型；认知领域仍停留在契约层面。",
      "de-DE":
        "Erfordert Lesemodelle für subjektive Szene und Überzeugung; Kognition ist auf Vertragsebene.",
      "es-ES":
        "Requiere modelos de lectura de escena subjetiva y de creencia; la cognición está a nivel de contrato.",
    },
  ),
  awaiting(
    "language",
    "cognition",
    "mana",
    {
      "en-US": "Language",
      "ru-RU": "Язык",
      "zh-Hans": "语言",
      "de-DE": "Sprache",
      "es-ES": "Lengua",
    },
    {
      "en-US": "Lexeme spread and semantic drift across the territory.",
      "ru-RU": "Распространение лексем и семантический дрейф по территории.",
      "zh-Hans": "词位在该地域上的扩散与语义漂移。",
      "de-DE": "Ausbreitung von Lexemen und semantische Drift über das Gebiet.",
      "es-ES": "Difusión de lexemas y deriva semántica a lo largo del territorio.",
    },
    {
      "en-US": "The language domain is not coupled to the runtime; there is nothing to project.",
      "ru-RU": "Языковой домен не связан со средой; проецировать нечего.",
      "zh-Hans": "语言领域尚未与运行时耦合；无可投影之物。",
      "de-DE":
        "Die Sprachdomäne ist nicht an die Laufzeitumgebung gekoppelt; es gibt nichts zu projizieren.",
      "es-ES":
        "El dominio de la lengua no está acoplado al entorno de ejecución; no hay nada que proyectar.",
    },
  ),

  /* ------------------------------------------------------------ society -- */
  awaiting(
    "social",
    "society",
    "life",
    {
      "en-US": "Social structure",
      "ru-RU": "Социальная структура",
      "zh-Hans": "社会结构",
      "de-DE": "Sozialstruktur",
      "es-ES": "Estructura social",
    },
    {
      "en-US": "Ties, groups and institutions placed in space.",
      "ru-RU": "Связи, группы и институты, размещённые в пространстве.",
      "zh-Hans": "置于空间中的联系、群体与制度。",
      "de-DE": "Bindungen, Gruppen und Institutionen im Raum verortet.",
      "es-ES": "Vínculos, grupos e instituciones situados en el espacio.",
    },
    {
      "en-US": "Needs agent-inferred structure and a read model.",
      "ru-RU": "Нужны выводимая агентами структура и модель чтения.",
      "zh-Hans": "需要由智能体推断出的结构以及一个读取模型。",
      "de-DE": "Benötigt von Agenten erschlossene Struktur und ein Lesemodell.",
      "es-ES": "Necesita estructura inferida por los agentes y un modelo de lectura.",
    },
  ),
  awaiting(
    "practices",
    "society",
    "physical",
    {
      "en-US": "Practices",
      "ru-RU": "Практики",
      "zh-Hans": "实践",
      "de-DE": "Praktiken",
      "es-ES": "Prácticas",
    },
    {
      "en-US": "Transmission and mutation of practices between places.",
      "ru-RU": "Передача и мутации практик между местами.",
      "zh-Hans": "实践在不同地点之间的传递与变异。",
      "de-DE": "Weitergabe und Wandel von Praktiken zwischen Orten.",
      "es-ES": "Transmisión y mutación de prácticas entre lugares.",
    },
    {
      "en-US": "Requires embodied practice execution, which does not exist yet.",
      "ru-RU": "Требуется воплощённое исполнение практик, которого пока нет.",
      "zh-Hans": "需要具身的实践执行，而这尚不存在。",
      "de-DE": "Erfordert verkörperte Ausführung von Praktiken, die es noch nicht gibt.",
      "es-ES": "Requiere ejecución encarnada de prácticas, que todavía no existe.",
    },
  ),
  awaiting(
    "economy",
    "society",
    "mana",
    {
      "en-US": "Economy",
      "ru-RU": "Хозяйство",
      "zh-Hans": "经济",
      "de-DE": "Wirtschaft",
      "es-ES": "Economía",
    },
    {
      "en-US": "Material flows, stocks and exchange between places.",
      "ru-RU": "Потоки материала, запасы и обмен между местами.",
      "zh-Hans": "地点之间的物质流动、存量与交换。",
      "de-DE": "Materialflüsse, Bestände und Austausch zwischen Orten.",
      "es-ES": "Flujos de material, existencias e intercambio entre lugares.",
    },
    {
      "en-US": "The city and material domains have no observer read model.",
      "ru-RU": "Городской и материальный домены не имеют модели чтения для наблюдателя.",
      "zh-Hans": "城市领域与物质领域都没有面向观测器的读取模型。",
      "de-DE": "Die Stadt- und Materiedomänen haben kein Beobachter-Lesemodell.",
      "es-ES": "Los dominios de ciudad y de materia no tienen modelo de lectura para el observador.",
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
