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

import { FieldRasterKind } from "@causafera/observer-protocol";

import { copyFor } from "../i18n";
import {
  formatCompact,
  formatInteger,
  formatMillimetresAsMetres,
  formatWaterVolume,
} from "../observer/format";
import { CHUNK_SIZE } from "../observer/models";
import { contourLevels, contourLines, refineField, type ChartField } from "./field";
import {
  EMPTY_LAYERS,
  fieldFrom,
  type Lens,
  type LensCellMark,
  type LensContext,
  type LensLayers,
  type LensSurface,
  type LensSymbol,
} from "./lens";
import { previewGradient, previewIsolines } from "./preview";
import {
  cellsChangedBy,
  columnField,
  rasterGeneration,
  receivedEdge,
  surfaceField,
  unsignedPeak,
  unsignedSurfaceField,
  type ColumnReading,
} from "./rasterFields";
import {
  MANA_STYLE,
  MOISTURE_STYLE,
  RELIEF_STYLE,
  TEXTURE_STYLE,
  WATER_STYLE,
  type SurfaceStyle,
} from "./surface";

/**
 * The lattice edge at which a field no longer has to be upsampled to be drawn.
 *
 * Below it the map is showing more resolution than it received and says so; at
 * or above it the samples are dense enough to read directly. The threshold is
 * compared against the received edge, so raising `chunk_extent` promotes the
 * mana lens with no change here.
 */
const DIRECTLY_DRAWABLE_EDGE = 8;

function manaAvailability(context: LensContext): Lens["availability"] {
  const edge = receivedEdge(context.rasters, FieldRasterKind.ManaIntensity);
  if (edge === 0) return "partial";
  return edge >= DIRECTLY_DRAWABLE_EDGE ? "observed" : "preview";
}

/** The mana field in plan view, under whichever reading the lens names. */
function manaSurface(context: LensContext, reading: ColumnReading): LensSurface | undefined {
  const field = columnField(context.rasters, FieldRasterKind.ManaIntensity, reading);
  if (field === undefined) return undefined;
  return {
    signature: `mana-${reading}:${field.min}:${field.max}:${field.patches}:${field.edge}`,
    field,
    style: MANA_STYLE,
    format: (value) => formatCompact(Math.round(value), context.locale),
  };
}

/**
 * Contours over an assembled field, at a round interval in the field's own unit.
 *
 * A coarse lattice is refined first so the line follows the same interpolation
 * the surface is painted with; a lattice already dense enough is traced directly.
 */
function measuredIsolines(
  field: ChartField,
  target: number,
  unit: number,
  label: (value: number) => string,
) {
  const traced = refineField(field, field.edge >= 16 ? 1 : Math.ceil(16 / field.edge));
  return contourLines(traced, contourLevels(field, target, unit)).map((line) => ({
    points: line.points,
    level: line.level,
    ordinal: line.ordinal,
    label: label(line.value),
    value: line.value,
  }));
}

function reliefSurface(context: LensContext): LensSurface | undefined {
  const field = surfaceField(context.rasters, FieldRasterKind.TerrainElevation);
  if (field === undefined) return undefined;
  return {
    signature: `relief:${field.min}:${field.max}:${field.patches}`,
    field,
    style: RELIEF_STYLE,
    unit: 1_000,
    format: (value) => `${formatMillimetresAsMetres(value, 1)} ${metreMark(context)}`,
  };
}

/**
 * One water bucket, as a surface over the charted extent.
 *
 * The three buckets are the same quantity in three places, so they share one
 * builder and differ only in which lattice they read and how they are painted.
 * The signature carries the peak as an exact count rather than the field's
 * double, so a tick that moved water past the precision of the painted value
 * still repaints.
 */
function waterSurface(
  context: LensContext,
  kind: FieldRasterKind,
  style: SurfaceStyle,
  name: string,
): LensSurface | undefined {
  const field = unsignedSurfaceField(context.rasters, kind);
  if (field === undefined) return undefined;
  const peak = unsignedPeak(context.rasters, kind) ?? 0n;
  return {
    // The generation trace rather than the extremes alone: water that moves
    // between two cells inside the current range changes neither min nor max,
    // and a signature that missed that would leave the last image on the chart.
    signature: `${name}:${rasterGeneration(context.rasters, kind)}:${peak}:${field.patches}`,
    field,
    style,
    // Rounded, because the painted field is interpolated between cells: a
    // fractional cubic millimetre between two samples is the drawing's, not the
    // world's, and printing it would dress an interpolation as a measurement.
    format: (value) => formatWaterVolume(BigInt(Math.round(value)), context.locale),
  };
}

/**
 * A water lens is observed once its lattice has arrived, and partial before it.
 *
 * Partial rather than awaiting, in the same sense the mana lens uses it: the
 * read model exists and the whole-session totals are already real data, so what
 * is missing before the first raster is the per-cell slice and not the domain.
 * A session that holds no water draws nothing under any of these, and the
 * caveat says which of the two is happening.
 */
function waterAvailability(kind: FieldRasterKind) {
  return (context: LensContext): Lens["availability"] =>
    unsignedSurfaceField(context.rasters, kind) === undefined ? "partial" : "observed";
}

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

/**
 * Water is carried as cubic millimetres and read as cubic metres.
 *
 * The regrouping is exact and the symbol is the same everywhere, so this is a
 * unit rather than a translation — but it still travels through the dictionary
 * shape, because a locale that writes it differently must be able to.
 */
const CUBIC_METRES: Lens["unit"] = {
  "en-US": "m³",
  "ru-RU": "м³",
  "zh-Hans": "立方米",
  "de-DE": "m³",
  "es-ES": "m³",
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
      "en-US": "Measured elevation per cell, hypsometrically tinted and shaded.",
      "ru-RU": "Измеренная высота по ячейкам, гипсометрическая окраска и отмывка.",
      "zh-Hans": "逐单元格的实测高程，配以等高彩色分层与晕渲。",
      "de-DE": "Gemessene Höhe je Zelle, hypsometrisch getönt und schattiert.",
      "es-ES": "Elevación medida por celda, con tintado hipsométrico y sombreado.",
    },
    unit: METRES,
    cellProjection: "full",
    caveat: {
      "en-US":
        "Shading is a lighting choice made by the observer, and the tint between samples is interpolated. Elevation steps at a chunk boundary are world state, not a seam in the drawing.",
      "ru-RU":
        "Отмывка — выбор освещения наблюдателем, а окраска между отсчётами интерполирована. Скачок высоты на границе чанка — это состояние мира, а не шов рисунка.",
      "zh-Hans":
        "晕渲是观测器所作的光照选择，采样点之间的着色为插值结果。区块边界处的高程落差属于世界状态，而非绘图接缝。",
      "de-DE":
        "Die Schattierung ist eine Lichtwahl des Beobachters, und die Tönung zwischen den Messpunkten ist interpoliert. Ein Höhensprung an einer Chunk-Grenze ist Weltzustand, keine Naht der Zeichnung.",
      "es-ES":
        "El sombreado es una elección de iluminación del observador, y el tintado entre muestras está interpolado. Un salto de elevación en el límite de un bloque es estado del mundo, no una costura del dibujo.",
    },
    layers: (context) => ({
      surface: reliefSurface(context),
      // Until a raster arrives the chunk aggregate is all there is, and one
      // value over one area is honestly drawn as one tint.
      field:
        reliefSurface(context) === undefined
          ? fieldFrom(
              context.atlas.chunks,
              (chunk) => chunk.maximumElevationMm,
              (value) => `${formatMillimetresAsMetres(value, 1)} ${metreMark(context)}`,
            )
          : undefined,
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
      "en-US": "Measured surface roughness per cell.",
      "ru-RU": "Измеренная шероховатость поверхности по ячейкам.",
      "zh-Hans": "逐单元格的实测表面粗糙度。",
      "de-DE": "Gemessene Oberflächenrauheit je Zelle.",
      "es-ES": "Rugosidad de la superficie medida por celda.",
    },
    unit: MILLIMETRES,
    cellProjection: "full",
    layers: (context) => {
      const field = surfaceField(
        context.rasters,
        FieldRasterKind.TerrainElevation,
        "auxiliary",
      );
      if (field === undefined) {
        return {
          field: fieldFrom(
            context.atlas.chunks,
            (chunk) => chunk.meanRoughnessMm,
            (value) => `${Math.round(value)} ${millimetreMark(context)}`,
          ),
        };
      }
      return {
        surface: {
          signature: `roughness:${field.min}:${field.max}:${field.patches}`,
          field,
          style: TEXTURE_STYLE,
          format: (value) => `${Math.round(value)} ${millimetreMark(context)}`,
        },
      };
    },
  },
  {
    id: "contours",
    group: "geography",
    signal: "physical",
    availability: "observed",
    roles: ["overlay"],
    title: {
      "en-US": "Contours",
      "ru-RU": "Изогипсы",
      "zh-Hans": "等高线",
      "de-DE": "Höhenlinien",
      "es-ES": "Curvas de nivel",
    },
    detail: {
      "en-US": "Contours traced through the measured elevation lattice.",
      "ru-RU": "Изогипсы, проведённые по измеренной решётке высот.",
      "zh-Hans": "沿实测高程格网追踪出的等高线。",
      "de-DE": "Höhenlinien, dem gemessenen Höhengitter entlang gezogen.",
      "es-ES": "Curvas trazadas a través de la retícula de elevación medida.",
    },
    caveat: {
      "en-US":
        "Every vertex lies between two measured cells. Where no raster has arrived the lens falls back to interpolating chunk values, and says so in the legend.",
      "ru-RU":
        "Каждая вершина лежит между двумя измеренными ячейками. Там, где растр не получен, линза переходит к интерполяции значений чанков и сообщает об этом в легенде.",
      "zh-Hans":
        "每个顶点都位于两个实测单元格之间。若尚未收到栅格，该透镜会退回到对区块数值的插值，并在图例中说明。",
      "de-DE":
        "Jeder Stützpunkt liegt zwischen zwei gemessenen Zellen. Wo kein Raster eingetroffen ist, weicht die Linse auf interpolierte Chunk-Werte aus und sagt es in der Legende.",
      "es-ES":
        "Cada vértice está entre dos celdas medidas. Donde no ha llegado ningún ráster, la lente recurre a interpolar valores de bloque y lo indica en la leyenda.",
    },
    cellProjection: "full",
    layers: (context) => {
      const field = surfaceField(context.rasters, FieldRasterKind.TerrainElevation);
      if (field === undefined) {
        return {
          isolines: previewIsolines(
            context.atlas.chunks,
            (chunk) => chunk.maximumElevationMm,
            9,
            (value) => `${formatMillimetresAsMetres(value, 0)} ${metreMark(context)}`,
          ),
        };
      }
      return {
        isolines: measuredIsolines(field, 9, 1_000, (value) =>
          `${formatMillimetresAsMetres(value, 0)} ${metreMark(context)}`,
        ),
      };
    },
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

  /* ---------------------------------------------------------- hydrology -- */
  {
    id: "water-surface",
    group: "hydrology",
    signal: "water",
    availability: "observed",
    availabilityFor: waterAvailability(FieldRasterKind.HydrologySurfaceWater),
    roles: ["primary", "overlay"],
    title: {
      "en-US": "Surface water",
      "ru-RU": "Поверхностная вода",
      "zh-Hans": "地表水",
      "de-DE": "Oberflächenwasser",
      "es-ES": "Agua superficial",
    },
    detail: {
      "en-US": "Water ponded above the ground, as an exact volume per cell.",
      "ru-RU": "Вода, стоящая над поверхностью, — точный объём по ячейкам.",
      "zh-Hans": "积聚在地表之上的水，逐单元格的精确体积。",
      "de-DE": "Über dem Boden stehendes Wasser, als exaktes Volumen je Zelle.",
      "es-ES": "Agua embalsada sobre el terreno, como volumen exacto por celda.",
    },
    unit: CUBIC_METRES,
    cellProjection: "full",
    caveat: {
      "en-US":
        "A volume, never a depth: a depth is a volume over a cell area, and the grid metric carrying that area is not projected to the observer. Where the surface is drawn between two cells the value is interpolated. There is no lake and no river here — a body of water is a reading a viewer may take, not simulation state.",
      "ru-RU":
        "Объём, а не глубина: глубина — это объём, делённый на площадь ячейки, а метрика решётки с этой площадью наблюдателю не передаётся. Между двумя ячейками значение интерполировано. Здесь нет ни озера, ни реки — водоём это прочтение зрителя, а не состояние симуляции.",
      "zh-Hans":
        "这是体积，而不是深度：深度是体积除以单元格面积，而携带该面积的格网度量并不投影给观测器。两个单元格之间的取值为插值结果。这里没有湖泊也没有河流——水体是观看者可作的读法，而非仿真状态。",
      "de-DE":
        "Ein Volumen, keine Tiefe: eine Tiefe ist ein Volumen je Zellfläche, und die Gittermetrik, die diese Fläche trägt, wird dem Beobachter nicht projiziert. Zwischen zwei Zellen ist der Wert interpoliert. Hier gibt es weder See noch Fluss — ein Gewässer ist eine Lesart des Betrachters, kein Simulationszustand.",
      "es-ES":
        "Un volumen, nunca una profundidad: la profundidad es un volumen por área de celda, y la métrica de retícula que lleva esa área no se proyecta al observador. Entre dos celdas el valor está interpolado. Aquí no hay lago ni río: una masa de agua es una lectura del espectador, no estado de la simulación.",
    },
    layers: (context) => {
      const surface = waterSurface(
        context,
        FieldRasterKind.HydrologySurfaceWater,
        WATER_STYLE,
        "water-surface",
      );
      return surface === undefined ? EMPTY_LAYERS : { surface };
    },
  },
  {
    id: "water-soil",
    group: "hydrology",
    signal: "water",
    availability: "observed",
    availabilityFor: waterAvailability(FieldRasterKind.HydrologySoilWater),
    roles: ["primary"],
    title: {
      "en-US": "Soil water",
      "ru-RU": "Почвенная вода",
      "zh-Hans": "土壤水",
      "de-DE": "Bodenwasser",
      "es-ES": "Agua del suelo",
    },
    detail: {
      "en-US": "Water held in the unsaturated zone, as an exact volume per cell.",
      "ru-RU": "Вода, удерживаемая в зоне аэрации, — точный объём по ячейкам.",
      "zh-Hans": "保持在非饱和带中的水，逐单元格的精确体积。",
      "de-DE": "In der ungesättigten Zone gehaltenes Wasser, als exaktes Volumen je Zelle.",
      "es-ES": "Agua retenida en la zona no saturada, como volumen exacto por celda.",
    },
    unit: CUBIC_METRES,
    cellProjection: "full",
    caveat: {
      "en-US":
        "The substrate's capacities and its infiltration limit are not projected, so a cell that looks full and one that is full cannot be told apart here.",
      "ru-RU":
        "Ёмкости субстрата и предел инфильтрации не проецируются, поэтому ячейку, которая выглядит полной, здесь не отличить от полной.",
      "zh-Hans":
        "基质的容量与入渗上限不会被投影，因此在这里无法区分“看起来满了”的单元格与真正满了的单元格。",
      "de-DE":
        "Die Kapazitäten des Substrats und sein Infiltrationslimit werden nicht projiziert; eine Zelle, die voll aussieht, ist hier nicht von einer vollen zu unterscheiden.",
      "es-ES":
        "Las capacidades del sustrato y su límite de infiltración no se proyectan, así que aquí no se distingue una celda que parece llena de una que lo está.",
    },
    layers: (context) => {
      const surface = waterSurface(
        context,
        FieldRasterKind.HydrologySoilWater,
        MOISTURE_STYLE,
        "water-soil",
      );
      return surface === undefined ? EMPTY_LAYERS : { surface };
    },
  },
  {
    id: "water-groundwater",
    group: "hydrology",
    signal: "water",
    availability: "observed",
    availabilityFor: waterAvailability(FieldRasterKind.HydrologyGroundwater),
    roles: ["primary"],
    title: {
      "en-US": "Groundwater",
      "ru-RU": "Подземная вода",
      "zh-Hans": "地下水",
      "de-DE": "Grundwasser",
      "es-ES": "Agua subterránea",
    },
    detail: {
      "en-US": "Water in the saturated zone, as an exact volume per cell.",
      "ru-RU": "Вода в зоне насыщения — точный объём по ячейкам.",
      "zh-Hans": "饱和带中的水，逐单元格的精确体积。",
      "de-DE": "Wasser in der gesättigten Zone, als exaktes Volumen je Zelle.",
      "es-ES": "Agua en la zona saturada, como volumen exacto por celda.",
    },
    unit: CUBIC_METRES,
    cellProjection: "full",
    caveat: {
      "en-US":
        "A stored volume, not a water table. The table is that volume against the cell's specific yield and aquifer base, and neither is projected — so this shows how much is down there and not how high it stands.",
      "ru-RU":
        "Запасённый объём, а не уровень грунтовых вод. Уровень — это тот же объём относительно водоотдачи ячейки и подошвы водоносного горизонта, а их не проецируют; здесь видно, сколько воды внизу, но не на какой она высоте.",
      "zh-Hans":
        "这是储存体积，而不是地下水位。水位是该体积相对于单元格给水度与含水层底板的结果，而二者都不被投影——所以这里显示的是下面有多少水，而不是它站得多高。",
      "de-DE":
        "Ein gespeichertes Volumen, kein Grundwasserspiegel. Der Spiegel ergibt sich aus diesem Volumen gegen den nutzbaren Porenraum der Zelle und die Aquiferbasis, und beides wird nicht projiziert — das hier zeigt, wie viel unten liegt, nicht wie hoch es steht.",
      "es-ES":
        "Un volumen almacenado, no un nivel freático. El nivel es ese volumen frente al rendimiento específico de la celda y la base del acuífero, y ninguno se proyecta: esto muestra cuánta agua hay abajo, no a qué altura está.",
    },
    layers: (context) => {
      const surface = waterSurface(
        context,
        FieldRasterKind.HydrologyGroundwater,
        MOISTURE_STYLE,
        "water-groundwater",
      );
      return surface === undefined ? EMPTY_LAYERS : { surface };
    },
  },
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
    availabilityFor: manaAvailability,
    roles: ["primary"],
    title: {
      "en-US": "Mana field",
      "ru-RU": "Поле маны",
      "zh-Hans": "魔力场",
      "de-DE": "Mana-Feld",
      "es-ES": "Campo de maná",
    },
    detail: {
      "en-US": "Measured mana intensity, summed through each column of the lattice.",
      "ru-RU": "Измеренная интенсивность маны, просуммированная по каждому столбцу решётки.",
      "zh-Hans": "实测魔力强度，沿格网每一列求和。",
      "de-DE": "Gemessene Mana-Intensität, über jede Säule des Gitters summiert.",
      "es-ES": "Intensidad de maná medida, sumada a lo largo de cada columna de la retícula.",
    },
    cellProjection: "full",
    caveat: {
      "en-US":
        "The column sum is one reading of a volumetric field; the peak lens reads the same volume differently. The lattice is coarser than the drawn surface, which is upsampled between samples.",
      "ru-RU":
        "Сумма по столбцу — одно из прочтений объёмного поля; линза пика читает тот же объём иначе. Решётка грубее нарисованной поверхности, которая интерполирована между отсчётами.",
      "zh-Hans":
        "列求和只是对体场的一种读法；峰值透镜以另一种方式读取同一体积。格网比所绘表面更粗，采样点之间经过了上采样。",
      "de-DE":
        "Die Säulensumme ist eine Lesart eines volumetrischen Feldes; die Spitzenlinse liest dasselbe Volumen anders. Das Gitter ist gröber als die gezeichnete Fläche, die zwischen den Messpunkten hochgerechnet wird.",
      "es-ES":
        "La suma por columna es una lectura de un campo volumétrico; la lente de pico lee el mismo volumen de otro modo. La retícula es más gruesa que la superficie dibujada, que se sobremuestrea entre muestras.",
    },
    layers: (context) => {
      const surface = manaSurface(context, "sum");
      if (surface === undefined) {
        return {
          field: fieldFrom(
            context.atlas.chunks,
            (chunk) => chunk.manaTotal,
            (value) => formatCompact(value, context.locale),
          ),
        };
      }
      return { surface };
    },
  },
  {
    id: "mana-peak",
    group: "mana",
    signal: "mana",
    availability: "observed",
    availabilityFor: manaAvailability,
    roles: ["primary"],
    title: {
      "en-US": "Mana peak",
      "ru-RU": "Пик маны",
      "zh-Hans": "魔力峰值",
      "de-DE": "Mana-Spitze",
      "es-ES": "Pico de maná",
    },
    detail: {
      "en-US": "The most intense cell anywhere in each column of the lattice.",
      "ru-RU": "Наиболее интенсивная ячейка в каждом столбце решётки.",
      "zh-Hans": "格网每一列中强度最高的单元格。",
      "de-DE": "Die intensivste Zelle irgendwo in jeder Säule des Gitters.",
      "es-ES": "La celda más intensa en cualquier punto de cada columna de la retícula.",
    },
    cellProjection: "full",
    caveat: {
      "en-US":
        "A maximum answers where the field gets strongest, not how much stands over the ground; the field lens reads the same volume as a sum.",
      "ru-RU":
        "Максимум отвечает, где поле сильнее всего, а не сколько его стоит над землёй; линза поля читает тот же объём как сумму.",
      "zh-Hans":
        "最大值回答的是场在何处最强，而不是地面之上有多少；场透镜将同一体积读作总和。",
      "de-DE":
        "Ein Maximum beantwortet, wo das Feld am stärksten wird, nicht wie viel über dem Boden steht; die Feldlinse liest dasselbe Volumen als Summe.",
      "es-ES":
        "Un máximo responde dónde el campo es más intenso, no cuánto se alza sobre el terreno; la lente de campo lee el mismo volumen como suma.",
    },
    layers: (context) => {
      const surface = manaSurface(context, "maximum");
      return surface === undefined ? EMPTY_LAYERS : { surface };
    },
  },
  {
    id: "mana-isolines",
    group: "mana",
    signal: "mana",
    availability: "observed",
    availabilityFor: manaAvailability,
    roles: ["overlay"],
    title: {
      "en-US": "Mana isolines",
      "ru-RU": "Изолинии маны",
      "zh-Hans": "魔力等值线",
      "de-DE": "Mana-Isolinien",
      "es-ES": "Isolíneas de maná",
    },
    detail: {
      "en-US": "Lines of equal column intensity, at stated levels.",
      "ru-RU": "Линии равной интенсивности столбца на указанных уровнях.",
      "zh-Hans": "等列强度线，标注所在层级。",
      "de-DE": "Linien gleicher Säulenintensität auf angegebenen Stufen.",
      "es-ES": "Líneas de igual intensidad por columna, en niveles declarados.",
    },
    caveat: {
      "en-US": "Traced through the received lattice, which is coarse; the levels are labelled.",
      "ru-RU": "Проведены по полученной решётке, которая груба; уровни подписаны.",
      "zh-Hans": "沿收到的格网追踪，该格网较粗；各层级均有标注。",
      "de-DE":
        "Durch das empfangene Gitter gezogen, das grob ist; die Stufen sind beschriftet.",
      "es-ES": "Trazadas a través de la retícula recibida, que es gruesa; los niveles se rotulan.",
    },
    cellProjection: "full",
    layers: (context) => {
      const field = columnField(context.rasters, FieldRasterKind.ManaIntensity, "sum");
      if (field === undefined) return EMPTY_LAYERS;
      return {
        isolines: measuredIsolines(field, 6, 1, (value) =>
          formatCompact(Math.round(value), context.locale),
        ),
      };
    },
  },
  {
    id: "mana-provenance",
    group: "mana",
    signal: "trace",
    availability: "observed",
    roles: ["overlay"],
    title: {
      "en-US": "Traced cells",
      "ru-RU": "Ячейки трассы",
      "zh-Hans": "受迹线影响的单元格",
      "de-DE": "Verfolgte Zellen",
      "es-ES": "Celdas trazadas",
    },
    detail: {
      "en-US": "The mana cells a followed trace last changed.",
      "ru-RU": "Ячейки маны, которые последний раз изменила отслеживаемая трасса.",
      "zh-Hans": "所跟踪迹线最后一次改变的魔力单元格。",
      "de-DE": "Die Mana-Zellen, die eine verfolgte Spur zuletzt verändert hat.",
      "es-ES": "Las celdas de maná que una traza seguida cambió por última vez.",
    },
    caveat: {
      "en-US":
        "Only the latest change is recorded per cell, so a trace that has since been overwritten leaves nothing. Select a trace anywhere in the interface to light its ground.",
      "ru-RU":
        "На ячейку записывается только последнее изменение, поэтому перекрытая трасса не оставляет ничего. Выберите трассу в любом месте интерфейса, чтобы подсветить её землю.",
      "zh-Hans":
        "每个单元格只记录最近一次变更，因此已被覆盖的迹线不会留下任何痕迹。在界面任意处选择一条迹线即可点亮其所在地面。",
      "de-DE":
        "Je Zelle wird nur die letzte Änderung festgehalten; eine seither überschriebene Spur hinterlässt nichts. Wählen Sie irgendwo in der Oberfläche eine Spur, um ihren Boden aufleuchten zu lassen.",
      "es-ES":
        "Sólo se registra el último cambio por celda, así que una traza ya sobrescrita no deja nada. Seleccione una traza en cualquier parte de la interfaz para iluminar su terreno.",
    },
    cellProjection: "full",
    layers: (context) => {
      if (context.traceFilter === undefined) return EMPTY_LAYERS;
      const cells = cellsChangedBy(
        context.rasters,
        FieldRasterKind.ManaIntensity,
        context.traceFilter,
      );
      return {
        cells: cells.map((cell) => ({
          chunkKey: `${cell.chunkX}:${cell.chunkY}:0`,
          chunkX: cell.chunkX,
          chunkY: cell.chunkY,
          // The mana lattice is coarser than the cell lattice the marks are
          // drawn on, so a mana cell is placed at the centre of the ground it
          // covers rather than pretending to be one terrain cell.
          cellX: Math.round(((cell.x + 0.5) / cell.edge) * CHUNK_SIZE - 0.5),
          cellY: Math.round(((cell.y + 0.5) / cell.edge) * CHUNK_SIZE - 0.5),
          cellZ: cell.z,
          intensity: 1,
          shape: "ring" as const,
          label: `z${cell.z}`,
        })),
      };
    },
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

/**
 * The chart opens on measured relief with water laid over it.
 *
 * Water needs ground under it to mean anything: the surface lens paints only
 * where water stands and is fully transparent where none does, so over the
 * hypsometric relief it reads the way water reads on a chart — a shape against
 * land, pooled in the low ground the solver actually routed it into. Contours
 * complete the reading by giving the elevation the water is answering to; they
 * no longer bunch at every seam now that terrain is continuous across chunk
 * boundaries (TODO-GEO-005), which is what previously kept them out of the
 * default set.
 *
 * The mana field is one lens click away and keeps every lens it had. It opened
 * the chart while it was the only continuous field the runtime maintained; that
 * is no longer the case.
 */
export const DEFAULT_PRIMARY_LENS = "relief";
export const DEFAULT_OVERLAYS = ["water-surface", "contours", "population"];

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
