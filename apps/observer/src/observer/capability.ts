/**
 * The capability register.
 *
 * The observer is an instrument pointed at a simulation whose domains are at very different
 * depths. Rather than hiding that, the register states — for every observable the project
 * has defined — whether the instrument can currently read it, and if not, what is missing:
 * an observer projection, or simulation depth.
 *
 * This is presentation content. The labels are observer classifications and carry no
 * simulation meaning (INV-006, INV-013). Entries are text so the register stays readable in
 * one place; UI chrome strings live in `src/i18n`.
 *
 * Maturity levels follow `docs/architecture/detailed-development-rebaseline.md`:
 * M0 documented · M1 contracted · M2 executable · M3 coupled · M4 observable · M5 validated.
 */

import type { ObserverLocale } from "./format";
import type { AreaId } from "../workspace";

export type CapabilityState =
  | "live" //             real data through the current protocol, rendered here
  | "bounded" //          real data, but restricted to a bounded window or configuration
  | "absent-projection" // the runtime holds it; no observer query or wire encoding exists
  | "absent-domain"; //   the simulation domain is not deep enough to project anything

export interface CapabilityEntry {
  id: string;
  state: CapabilityState;
  /** Backend maturity, then observer maturity. */
  domainMaturity: number;
  observerMaturity: number;
  title: Record<ObserverLocale, string>;
  detail: Record<ObserverLocale, string>;
  /** Area of the application that reads it, when something reads it. */
  area?: AreaId;
}

export interface CapabilityGroup {
  id: string;
  title: Record<ObserverLocale, string>;
  entries: CapabilityEntry[];
}

export const CAPABILITY_REGISTER: CapabilityGroup[] = [
  {
    id: "runtime",
    title: { "ru-RU": "Состояние среды", "en-US": "Runtime state" },
    entries: [
      {
        id: "runtime-summary",
        state: "live",
        domainMaturity: 3,
        observerMaturity: 4,
        area: "station",
        title: { "ru-RU": "Сводка среды", "en-US": "Runtime summary" },
        detail: {
          "ru-RU":
            "Двадцать три величины в потоке с якорями дайджестов: такты, мана, население, акторы, трассы, события.",
          "en-US":
            "Twenty-three streamed quantities with digest anchors: ticks, mana, population, actors, traces, events.",
        },
      },
      {
        id: "digests",
        state: "live",
        domainMaturity: 3,
        observerMaturity: 4,
        area: "station",
        title: { "ru-RU": "Якоря идентичности", "en-US": "Identity anchors" },
        detail: {
          "ru-RU":
            "Физический дайджест и дайджест истории проверяются при каждом обновлении. Это идентичность состояния, а не мера сходства.",
          "en-US":
            "Physical and history digests are verified on every update. They are state identity, never a similarity measure.",
        },
      },
      {
        id: "control",
        state: "live",
        domainMaturity: 3,
        observerMaturity: 4,
        area: "station",
        title: { "ru-RU": "Управление прогоном", "en-US": "Run control" },
        detail: {
          "ru-RU": "Шаг, пакет до 64 тактов, сброс с явным seed. Наблюдатель не изменяет состояние — он его продвигает.",
          "en-US":
            "Step, batches up to 64 ticks, reset with an explicit seed. The observer does not modify state; it advances the run.",
        },
      },
      {
        id: "metrics",
        state: "absent-projection",
        domainMaturity: 2,
        observerMaturity: 0,
        title: { "ru-RU": "Телеметрия производительности", "en-US": "Performance telemetry" },
        detail: {
          "ru-RU":
            "Схема PerformanceMetrics описана в протоколе, но проекции и кодирования нет. Панель приборов показывает только измеренную клиентом сторону.",
          "en-US":
            "The PerformanceMetrics schema exists in the protocol, but no read model or wire encoding does. The instrument panel shows only the client-measured side.",
        },
      },
    ],
  },
  {
    id: "space",
    title: { "ru-RU": "Пространство", "en-US": "Space" },
    entries: [
      {
        id: "chunks",
        state: "bounded",
        domainMaturity: 2,
        observerMaturity: 4,
        area: "chart",
        title: { "ru-RU": "Проекция чанков карты", "en-US": "Chart-qualified chunk projection" },
        detail: {
          "ru-RU":
            "Высота, шероховатость, мана, разрешение, население и события по активным чанкам. Координаты привязаны к карте: это не бесшовный глобус.",
          "en-US":
            "Elevation, roughness, mana, resolution, population and events per active chunk. Coordinates are chart-qualified: this is not a seamless globe.",
        },
      },
      {
        id: "surface-deltas",
        state: "bounded",
        domainMaturity: 3,
        observerMaturity: 4,
        area: "flux",
        title: { "ru-RU": "Переходы материальной поверхности", "en-US": "Material surface transitions" },
        detail: {
          "ru-RU":
            "Окно последних переходов состояния поверхности с трассами контакта, эффектов маны и локальной маны. Окно ограничено 64 записями.",
          "en-US":
            "A window of recent surface condition transitions with contact, mana-effect and local-mana traces. The window is bounded at 64 records.",
        },
      },
      {
        id: "gate-deltas",
        state: "bounded",
        domainMaturity: 3,
        observerMaturity: 4,
        area: "flux",
        title: { "ru-RU": "Затворы локальной маны", "en-US": "Local mana gates" },
        detail: {
          "ru-RU":
            "Наблюдатель получает только переходы в закрытое состояние. Пустой список означает отсутствие таких переходов, а не отсутствие затворов.",
          "en-US":
            "Only transitions into the closed state are projected. An empty list means no such transition occurred, not that gates are absent.",
        },
      },
      {
        id: "mana-field",
        state: "absent-projection",
        domainMaturity: 4,
        observerMaturity: 1,
        title: { "ru-RU": "Поле маны по ячейкам", "en-US": "Per-cell mana field" },
        detail: {
          "ru-RU":
            "Среда хранит поле по ячейкам; наблюдателю проецируются только суммы по чанку и пиковая интенсивность.",
          "en-US":
            "The runtime holds a per-cell field; only chunk totals and the peak cell intensity are projected to the observer.",
        },
      },
      {
        id: "resolution-field",
        state: "absent-projection",
        domainMaturity: 3,
        observerMaturity: 1,
        title: { "ru-RU": "Поле причинного разрешения", "en-US": "Causal resolution field" },
        detail: {
          "ru-RU":
            "По чанку доступны релевантность и уровень. Пороги переключения уровней политикой не проецируются, поэтому шкала показана в сырых единицах.",
          "en-US":
            "Relevance and level are available per chunk. The policy thresholds that separate levels are not projected, so the scale is shown in raw units.",
        },
      },
    ],
  },
  {
    id: "causality",
    title: { "ru-RU": "Причинность", "en-US": "Causality" },
    entries: [
      {
        id: "trace-anchors",
        state: "bounded",
        domainMaturity: 3,
        observerMaturity: 3,
        area: "flux",
        title: { "ru-RU": "Якоря трасс", "en-US": "Trace anchors" },
        detail: {
          "ru-RU":
            "Идентификаторы трасс приходят вместе с переходами и утверждениями и связывают наблюдаемое с провенансом. Сами трассы не запрашиваются.",
          "en-US":
            "Trace identifiers arrive with transitions and claims and tie an observation to its provenance. The traces themselves cannot be fetched.",
        },
      },
      {
        id: "trace-graph",
        state: "absent-projection",
        domainMaturity: 3,
        observerMaturity: 0,
        title: { "ru-RU": "Граф происхождения", "en-US": "Provenance graph" },
        detail: {
          "ru-RU":
            "CausalTraceStore хранит предков и обход, но вида запроса, кодирования и декодера для графа нет. Пока показаны только якоря.",
          "en-US":
            "CausalTraceStore holds ancestry and traversal, but no query kind, wire encoding or decoder exists for the graph. Only anchors are shown.",
        },
      },
      {
        id: "history",
        state: "absent-projection",
        domainMaturity: 2,
        observerMaturity: 0,
        title: { "ru-RU": "Исторические состояния", "en-US": "Historical state" },
        detail: {
          "ru-RU":
            "Запросов к сохранённой истории нет. Ряд на графиках собран наблюдателем из полученных кадров и авторитетным не является.",
          "en-US":
            "There are no queries against stored history. The series on these charts are assembled by the observer from received frames and are not authoritative.",
        },
      },
    ],
  },
  {
    id: "explanation",
    title: { "ru-RU": "Объяснение", "en-US": "Explanation" },
    entries: [
      {
        id: "explanation-ir",
        state: "bounded",
        domainMaturity: 3,
        observerMaturity: 4,
        area: "assay",
        title: { "ru-RU": "Типизированный Explanation IR", "en-US": "Typed Explanation IR" },
        detail: {
          "ru-RU":
            "Утверждения со схемой, значением, уверенностью, состоянием свидетельства и якорями трасс. Ограничен экспериментом материальной поверхности.",
          "en-US":
            "Claims with a schema, value, confidence, evidence state and trace anchors. Bounded to the material-surface experiment.",
        },
      },
      {
        id: "rendered-explanation",
        state: "absent-projection",
        domainMaturity: 4,
        observerMaturity: 1,
        title: { "ru-RU": "Детерминированный текст объяснения", "en-US": "Deterministic explanation text" },
        detail: {
          "ru-RU":
            "Шаблонный рендер существует в Rust и является авторитетным. Наблюдатель показывает структуру утверждений и не воспроизводит текст самостоятельно.",
          "en-US":
            "The template renderer exists in Rust and is authoritative. The observer presents claim structure and does not reproduce the text on its own.",
        },
      },
    ],
  },
  {
    id: "agents",
    title: { "ru-RU": "Агенты и общество", "en-US": "Agents and society" },
    entries: [
      {
        id: "entities",
        state: "absent-projection",
        domainMaturity: 2,
        observerMaturity: 0,
        title: { "ru-RU": "Сущности и акторы", "en-US": "Entities and actors" },
        detail: {
          "ru-RU":
            "Доступно только число акторов и суммарное население. Схема EntitySummary описана, но модели чтения нет.",
          "en-US":
            "Only the actor count and total population are available. The EntitySummary schema is defined, but no read model exists.",
        },
      },
      {
        id: "subjective",
        state: "absent-domain",
        domainMaturity: 1,
        observerMaturity: 0,
        title: { "ru-RU": "Субъективное знание", "en-US": "Subjective knowledge" },
        detail: {
          "ru-RU":
            "Всё показанное здесь — объективная проекция. Сцена, память и убеждения агентов находятся на уровне контрактов и не проецируются.",
          "en-US":
            "Everything shown here is an objective projection. Agent scene, memory and belief are at contract level and are not projected.",
        },
      },
      {
        id: "language",
        state: "absent-domain",
        domainMaturity: 1,
        observerMaturity: 0,
        title: { "ru-RU": "Язык", "en-US": "Language" },
        detail: {
          "ru-RU": "Лексемы и субъективные ассоциации описаны в протоколе, но домен ещё не связан со средой.",
          "en-US": "Lexemes and subjective associations exist in the protocol, but the domain is not coupled to the runtime yet.",
        },
      },
      {
        id: "social",
        state: "absent-domain",
        domainMaturity: 1,
        observerMaturity: 0,
        title: { "ru-RU": "Социальная структура", "en-US": "Social structure" },
        detail: {
          "ru-RU": "Контракты существуют; выводимой агентами структуры и модели чтения — нет.",
          "en-US": "Contracts exist; agent-inferred structure and a read model do not.",
        },
      },
      {
        id: "practices",
        state: "absent-domain",
        domainMaturity: 1,
        observerMaturity: 0,
        title: { "ru-RU": "Практики и концепты", "en-US": "Practices and concepts" },
        detail: {
          "ru-RU": "Передача практик и эволюция концептов требуют исполняемого воплощения, которого пока нет.",
          "en-US": "Practice transmission and concept evolution require embodied execution that does not exist yet.",
        },
      },
    ],
  },
];

const CAPABILITY_ENTRIES: CapabilityEntry[] = CAPABILITY_REGISTER.flatMap(
  (group) => group.entries,
);

export function capabilityCounts(): Record<CapabilityState, number> {
  const counts: Record<CapabilityState, number> = {
    live: 0,
    bounded: 0,
    "absent-projection": 0,
    "absent-domain": 0,
  };
  for (const entry of CAPABILITY_ENTRIES) counts[entry.state] += 1;
  return counts;
}

/** Capability identifiers advertised by the protocol handshake are query kinds. */
export const NEGOTIATED_CAPABILITY_NAMES: Record<number, string> = {
  1: "RuntimeSummary",
  2: "ExplanationIr",
  3: "WorldChunks",
};
