/**
 * Explanation claim descriptors.
 *
 * A claim arrives as an opaque `schemaId` plus a typed numeric value. The descriptors below
 * say how to read that value for the schemas the project has registered. They are
 * presentation only — an observer label is never simulation meaning (INV-006), and an
 * unrecognised schema is rendered generically rather than hidden, so new claim schemas
 * appear in this view the moment the backend emits them.
 *
 * The authoritative wording lives in the Rust deterministic renderer; this file deliberately
 * does not reproduce its sentences (see the capability register entry
 * `rendered-explanation`).
 */

import { Assessment, EvidenceState, type NumericClaimValue } from "@causafera/observer-protocol";

import type { SignalId } from "./models";
import type { ObserverLocale } from "./format";

export interface ClaimDescriptor {
  schemaId: number;
  signal: SignalId;
  title: Record<ObserverLocale, string>;
  /** What the numeric value means for this schema. */
  reading: Record<ObserverLocale, string>;
  /** Unit shown beside the value, when one applies. */
  unit?: Record<ObserverLocale, string>;
  /** A ratio of 1/1 means "observed"; used for the repetition control claim. */
  ratioIsFlag?: boolean;
}

export const CLAIM_DESCRIPTORS: Record<number, ClaimDescriptor> = {
  10: {
    schemaId: 10,
    signal: "physical",
    title: {
      "en-US": "Material surface loop",
      "ru-RU": "Контур материальной поверхности",
      "zh-Hans": "物质表面回路",
      "de-DE": "Materieller Oberflächenkreis",
      "es-ES": "Ciclo de superficie material",
    },
    reading: {
      "en-US":
        "The surface condition before and after the observed transition. Supported only when repeated structure and a mana effect are both present.",
      "ru-RU":
        "Диапазон состояния поверхности до и после наблюдаемого перехода. Подтверждается только при повторяемой структуре и наличии эффекта маны.",
      "zh-Hans":
        "所观测转变前后的表面状态。只有在同时存在重复结构与魔力效应时才获得支持。",
      "de-DE":
        "Der Oberflächenzustand vor und nach dem beobachteten Übergang. Nur gestützt, wenn wiederholte Struktur und ein Mana-Effekt zugleich vorliegen.",
      "es-ES":
        "El estado de la superficie antes y después de la transición observada. Sólo queda respaldado cuando hay a la vez estructura repetida y un efecto de maná.",
    },
    unit: {
      "en-US": "condition",
      "ru-RU": "состояние",
      "zh-Hans": "状态",
      "de-DE": "Zustand",
      "es-ES": "estado",
    },
  },
  11: {
    schemaId: 11,
    signal: "trace",
    title: {
      "en-US": "Observation window",
      "ru-RU": "Окно наблюдения",
      "zh-Hans": "观测窗口",
      "de-DE": "Beobachtungsfenster",
      "es-ES": "Ventana de observación",
    },
    reading: {
      "en-US": "The first and last tick of the interval the evidence was gathered over.",
      "ru-RU": "Такты начала и конца интервала, в котором собраны свидетельства.",
      "zh-Hans": "收集证据所跨区间的首刻与末刻。",
      "de-DE": "Der erste und der letzte Takt des Intervalls, über das die Belege gesammelt wurden.",
      "es-ES": "El primer y el último tic del intervalo en el que se reunió la evidencia.",
    },
    unit: {
      "en-US": "tick",
      "ru-RU": "такт",
      "zh-Hans": "刻",
      "de-DE": "Takt",
      "es-ES": "tic",
    },
  },
  12: {
    schemaId: 12,
    signal: "resolution",
    title: {
      "en-US": "Repetition control",
      "ru-RU": "Контроль повторяемости",
      "zh-Hans": "重复性对照",
      "de-DE": "Wiederholungskontrolle",
      "es-ES": "Control de repetición",
    },
    reading: {
      "en-US":
        "Whether repeated transition structure was observed. This is an experimental control, not a measured quantity.",
      "ru-RU":
        "Наблюдалась ли повторяющаяся структура переходов. Это контроль эксперимента, а не измеряемая величина.",
      "zh-Hans": "是否观测到重复的转变结构。这是实验对照，而不是被测量的量。",
      "de-DE":
        "Ob wiederholte Übergangsstruktur beobachtet wurde. Das ist eine Versuchskontrolle, keine gemessene Größe.",
      "es-ES":
        "Si se observó una estructura de transición repetida. Es un control experimental, no una magnitud medida.",
    },
    ratioIsFlag: true,
  },
  13: {
    schemaId: 13,
    signal: "mana",
    title: {
      "en-US": "Mana context",
      "ru-RU": "Контекст маны",
      "zh-Hans": "魔力背景",
      "de-DE": "Mana-Kontext",
      "es-ES": "Contexto de maná",
    },
    reading: {
      "en-US": "Total mana intensity accompanying the transition.",
      "ru-RU": "Суммарная интенсивность маны, сопутствующая переходу.",
      "zh-Hans": "伴随该转变的魔力总强度。",
      "de-DE": "Die Gesamtintensität des Mana, die den Übergang begleitet.",
      "es-ES": "Intensidad total de maná que acompaña a la transición.",
    },
    unit: {
      "en-US": "intensity",
      "ru-RU": "интенсивность",
      "zh-Hans": "强度",
      "de-DE": "Intensität",
      "es-ES": "intensidad",
    },
  },
  14: {
    schemaId: 14,
    signal: "mana",
    title: {
      "en-US": "Mana transition",
      "ru-RU": "Переход маны",
      "zh-Hans": "魔力转变",
      "de-DE": "Mana-Übergang",
      "es-ES": "Transición de maná",
    },
    reading: {
      "en-US":
        "Intensity before and after the transition. Remains unknown when no mana transition trace anchors the event.",
      "ru-RU":
        "Интенсивность до и после перехода. Остаётся неизвестной, если трасса перехода маны не привязана к событию.",
      "zh-Hans": "转变前后的强度。若没有魔力转变迹线锚定该事件，则保持未知。",
      "de-DE":
        "Die Intensität vor und nach dem Übergang. Bleibt unbekannt, solange keine Mana-Übergangsspur das Ereignis verankert.",
      "es-ES":
        "Intensidad antes y después de la transición. Permanece desconocida cuando ninguna traza de transición de maná ancla el suceso.",
    },
    unit: {
      "en-US": "intensity",
      "ru-RU": "интенсивность",
      "zh-Hans": "强度",
      "de-DE": "Intensität",
      "es-ES": "intensidad",
    },
  },
  15: {
    schemaId: 15,
    signal: "mana",
    title: {
      "en-US": "Local mana coupling",
      "ru-RU": "Связь локальной маны",
      "zh-Hans": "局部魔力耦合",
      "de-DE": "Kopplung des lokalen Mana",
      "es-ES": "Acoplamiento de maná local",
    },
    reading: {
      "en-US":
        "The local mana range at the surface across the gate transition. Anchored by the local mana and gate traces.",
      "ru-RU":
        "Диапазон локальной маны у поверхности вокруг перехода затвора. Опирается на трассы локальной маны и затвора.",
      "zh-Hans": "闸门转变前后表面处的局部魔力范围。由局部魔力迹线与闸门迹线锚定。",
      "de-DE":
        "Die Spanne des lokalen Mana an der Oberfläche über den Schleusenübergang hinweg. Verankert durch die Spuren des lokalen Mana und der Schleuse.",
      "es-ES":
        "El rango de maná local en la superficie a lo largo de la transición de compuerta. Anclado por las trazas de maná local y de compuerta.",
    },
    unit: {
      "en-US": "local mana",
      "ru-RU": "локальная мана",
      "zh-Hans": "局部魔力",
      "de-DE": "lokales Mana",
      "es-ES": "maná local",
    },
  },
};

export function claimDescriptor(schemaId: bigint): ClaimDescriptor | undefined {
  return CLAIM_DESCRIPTORS[Number(schemaId)];
}

/** A repetition-control ratio is a flag, so it reads as an outcome rather than as a number. */
const OBSERVED_FLAG: Record<ObserverLocale, string> = {
  "en-US": "observed",
  "ru-RU": "наблюдалось",
  "zh-Hans": "已观测到",
  "de-DE": "beobachtet",
  "es-ES": "observada",
};

const NOT_OBSERVED_FLAG: Record<ObserverLocale, string> = {
  "en-US": "not observed",
  "ru-RU": "не наблюдалось",
  "zh-Hans": "未观测到",
  "de-DE": "nicht beobachtet",
  "es-ES": "no observada",
};

/** Renders the typed value without asserting a meaning the schema does not carry. */
export function formatClaimValue(
  value: NumericClaimValue,
  descriptor: ClaimDescriptor | undefined,
  locale: ObserverLocale,
): string {
  if (value.kind === "scalar") return value.value.toString();
  if (value.kind === "range") return `${value.start} → ${value.end}`;
  if (descriptor?.ratioIsFlag === true && value.denominator === 1n) {
    const flag = value.numerator > 0n ? OBSERVED_FLAG : NOT_OBSERVED_FLAG;
    return flag[locale];
  }
  return `${value.numerator} / ${value.denominator}`;
}

/** Magnitude of change a range claim expresses, for the inline change indicator. */
export function claimDelta(value: NumericClaimValue): number | undefined {
  if (value.kind !== "range") return undefined;
  return Number(value.end - value.start);
}

export type StatusTone = "supported" | "partial" | "unsupported" | "unknown";

export function evidenceTone(state: EvidenceState): StatusTone {
  if (state === EvidenceState.Supported) return "supported";
  if (state === EvidenceState.Unsupported) return "unsupported";
  return "unknown";
}

export function assessmentTone(assessment: Assessment): StatusTone {
  if (assessment === Assessment.Supported) return "supported";
  if (assessment === Assessment.Partial) return "partial";
  if (assessment === Assessment.Unsupported) return "unsupported";
  return "unknown";
}

export const COMPARISON_CONTEXT: Record<number, Record<ObserverLocale, string>> = {
  0: {
    "en-US": "no comparison",
    "ru-RU": "без сравнения",
    "zh-Hans": "无对照",
    "de-DE": "kein Vergleich",
    "es-ES": "sin comparación",
  },
  1: {
    "en-US": "matched cohort",
    "ru-RU": "сопоставленная когорта",
    "zh-Hans": "匹配队列",
    "de-DE": "gepaarte Kohorte",
    "es-ES": "cohorte emparejada",
  },
  2: {
    "en-US": "counterfactual",
    "ru-RU": "контрфактическое сравнение",
    "zh-Hans": "反事实对照",
    "de-DE": "kontrafaktisch",
    "es-ES": "contrafactual",
  },
};
