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
      "ru-RU": "Контур материальной поверхности",
      "en-US": "Material surface loop",
    },
    reading: {
      "ru-RU":
        "Диапазон состояния поверхности до и после наблюдаемого перехода. Подтверждается только при повторяемой структуре и наличии эффекта маны.",
      "en-US":
        "The surface condition before and after the observed transition. Supported only when repeated structure and a mana effect are both present.",
    },
    unit: { "ru-RU": "состояние", "en-US": "condition" },
  },
  11: {
    schemaId: 11,
    signal: "trace",
    title: {
      "ru-RU": "Окно наблюдения",
      "en-US": "Observation window",
    },
    reading: {
      "ru-RU": "Такты начала и конца интервала, в котором собраны свидетельства.",
      "en-US": "The first and last tick of the interval the evidence was gathered over.",
    },
    unit: { "ru-RU": "такт", "en-US": "tick" },
  },
  12: {
    schemaId: 12,
    signal: "resolution",
    title: {
      "ru-RU": "Контроль повторяемости",
      "en-US": "Repetition control",
    },
    reading: {
      "ru-RU":
        "Наблюдалась ли повторяющаяся структура переходов. Это контроль эксперимента, а не измеряемая величина.",
      "en-US":
        "Whether repeated transition structure was observed. This is an experimental control, not a measured quantity.",
    },
    ratioIsFlag: true,
  },
  13: {
    schemaId: 13,
    signal: "mana",
    title: {
      "ru-RU": "Контекст маны",
      "en-US": "Mana context",
    },
    reading: {
      "ru-RU": "Суммарная интенсивность маны, сопутствующая переходу.",
      "en-US": "Total mana intensity accompanying the transition.",
    },
    unit: { "ru-RU": "интенсивность", "en-US": "intensity" },
  },
  14: {
    schemaId: 14,
    signal: "mana",
    title: {
      "ru-RU": "Переход маны",
      "en-US": "Mana transition",
    },
    reading: {
      "ru-RU":
        "Интенсивность до и после перехода. Остаётся неизвестной, если трасса перехода маны не привязана к событию.",
      "en-US":
        "Intensity before and after the transition. Remains unknown when no mana transition trace anchors the event.",
    },
    unit: { "ru-RU": "интенсивность", "en-US": "intensity" },
  },
  15: {
    schemaId: 15,
    signal: "mana",
    title: {
      "ru-RU": "Связь локальной маны",
      "en-US": "Local mana coupling",
    },
    reading: {
      "ru-RU":
        "Диапазон локальной маны у поверхности вокруг перехода затвора. Опирается на трассы локальной маны и затвора.",
      "en-US":
        "The local mana range at the surface across the gate transition. Anchored by the local mana and gate traces.",
    },
    unit: { "ru-RU": "локальная мана", "en-US": "local mana" },
  },
};

export function claimDescriptor(schemaId: bigint): ClaimDescriptor | undefined {
  return CLAIM_DESCRIPTORS[Number(schemaId)];
}

/** Renders the typed value without asserting a meaning the schema does not carry. */
export function formatClaimValue(
  value: NumericClaimValue,
  descriptor: ClaimDescriptor | undefined,
  locale: ObserverLocale,
): string {
  if (value.kind === "scalar") return value.value.toString();
  if (value.kind === "range") return `${value.start} → ${value.end}`;
  if (descriptor?.ratioIsFlag === true && value.denominator === 1n) {
    const observed = value.numerator > 0n;
    if (locale === "ru-RU") return observed ? "наблюдалось" : "не наблюдалось";
    return observed ? "observed" : "not observed";
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
  0: { "ru-RU": "без сравнения", "en-US": "no comparison" },
  1: { "ru-RU": "сопоставленная когорта", "en-US": "matched cohort" },
  2: { "ru-RU": "контрфактическое сравнение", "en-US": "counterfactual" },
};
