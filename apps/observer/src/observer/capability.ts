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
    title: {
      "en-US": "Runtime state",
      "ru-RU": "Состояние среды",
      "zh-Hans": "运行时状态",
      "de-DE": "Zustand der Laufzeitumgebung",
      "es-ES": "Estado del entorno de ejecución",
    },
    entries: [
      {
        id: "runtime-summary",
        state: "live",
        domainMaturity: 3,
        observerMaturity: 4,
        area: "station",
        title: {
          "en-US": "Runtime summary",
          "ru-RU": "Сводка среды",
          "zh-Hans": "运行时摘要",
          "de-DE": "Laufzeitübersicht",
          "es-ES": "Resumen del entorno",
        },
        detail: {
          "en-US":
            "Twenty-three streamed quantities with digest anchors: ticks, mana, population, actors, traces, events.",
          "ru-RU":
            "Двадцать три величины в потоке с якорями дайджестов: такты, мана, население, акторы, трассы, события.",
          "zh-Hans":
            "二十三项带摘要锚点的流式量：刻、魔力、人口、行动者、迹线、事件。",
          "de-DE":
            "Dreiundzwanzig gestreamte Größen mit Digest-Ankern: Takte, Mana, Bevölkerung, Akteure, Spuren, Ereignisse.",
          "es-ES":
            "Veintitrés magnitudes transmitidas con anclas de huella: tics, maná, población, actores, trazas, sucesos.",
        },
      },
      {
        id: "digests",
        state: "live",
        domainMaturity: 3,
        observerMaturity: 4,
        area: "station",
        title: {
          "en-US": "Identity anchors",
          "ru-RU": "Якоря идентичности",
          "zh-Hans": "身份锚点",
          "de-DE": "Identitätsanker",
          "es-ES": "Anclas de identidad",
        },
        detail: {
          "en-US":
            "Physical and history digests are verified on every update. They are state identity, never a similarity measure.",
          "ru-RU":
            "Физический дайджест и дайджест истории проверяются при каждом обновлении. Это идентичность состояния, а не мера сходства.",
          "zh-Hans":
            "每次更新都会校验物理摘要与历史摘要。它们是状态的身份标识，绝不是相似度度量。",
          "de-DE":
            "Physischer und Historien-Digest werden bei jeder Aktualisierung geprüft. Sie sind Zustandsidentität, niemals ein Ähnlichkeitsmaß.",
          "es-ES":
            "Las huellas física e histórica se verifican en cada actualización. Son la identidad del estado, nunca una medida de semejanza.",
        },
      },
      {
        id: "control",
        state: "live",
        domainMaturity: 3,
        observerMaturity: 4,
        area: "station",
        title: {
          "en-US": "Run control",
          "ru-RU": "Управление прогоном",
          "zh-Hans": "运行控制",
          "de-DE": "Laufsteuerung",
          "es-ES": "Control de la ejecución",
        },
        detail: {
          "en-US":
            "Step, batches up to 64 ticks, reset with an explicit seed. The observer does not modify state; it advances the run.",
          "ru-RU":
            "Шаг, пакет до 64 тактов, сброс с явным seed. Наблюдатель не изменяет состояние — он его продвигает.",
          "zh-Hans":
            "单步、最多 64 刻的批量推进、以显式 seed 重置。观测器不修改状态，只推进运行。",
          "de-DE":
            "Schritt, Bündel bis 64 Takte, Zurücksetzen mit ausdrücklichem Seed. Der Beobachter verändert keinen Zustand; er treibt den Lauf voran.",
          "es-ES":
            "Paso, lotes de hasta 64 tics, reinicio con un seed explícito. El observador no modifica el estado; hace avanzar la ejecución.",
        },
      },
      {
        id: "metrics",
        state: "absent-projection",
        domainMaturity: 2,
        observerMaturity: 0,
        title: {
          "en-US": "Performance telemetry",
          "ru-RU": "Телеметрия производительности",
          "zh-Hans": "性能遥测",
          "de-DE": "Leistungstelemetrie",
          "es-ES": "Telemetría de rendimiento",
        },
        detail: {
          "en-US":
            "The PerformanceMetrics schema exists in the protocol, but no read model or wire encoding does. The instrument panel shows only the client-measured side.",
          "ru-RU":
            "Схема PerformanceMetrics описана в протоколе, но проекции и кодирования нет. Панель приборов показывает только измеренную клиентом сторону.",
          "zh-Hans":
            "协议中已有 PerformanceMetrics 模式，但既无读取模型也无线路编码。仪表面板只显示客户端测得的一侧。",
          "de-DE":
            "Das PerformanceMetrics-Schema existiert im Protokoll, ein Lesemodell oder eine Drahtkodierung nicht. Die Instrumententafel zeigt nur die clientseitig gemessene Seite.",
          "es-ES":
            "El esquema PerformanceMetrics existe en el protocolo, pero no hay modelo de lectura ni codificación de transporte. El panel muestra sólo el lado medido por el cliente.",
        },
      },
    ],
  },
  {
    id: "space",
    title: {
      "en-US": "Space",
      "ru-RU": "Пространство",
      "zh-Hans": "空间",
      "de-DE": "Raum",
      "es-ES": "Espacio",
    },
    entries: [
      {
        id: "chunks",
        state: "bounded",
        domainMaturity: 2,
        observerMaturity: 4,
        area: "chart",
        title: {
          "en-US": "Chart-qualified chunk projection",
          "ru-RU": "Проекция чанков карты",
          "zh-Hans": "以图幅为限定的区块投影",
          "de-DE": "Kartenblattgebundene Chunk-Projektion",
          "es-ES": "Proyección de bloques referida a la carta",
        },
        detail: {
          "en-US":
            "Elevation, roughness, mana, resolution, population and events per active chunk. Coordinates are chart-qualified: this is not a seamless globe.",
          "ru-RU":
            "Высота, шероховатость, мана, разрешение, население и события по активным чанкам. Координаты привязаны к карте: это не бесшовный глобус.",
          "zh-Hans":
            "每个活跃区块的高程、粗糙度、魔力、分辨率、人口与事件。坐标以图幅为限定：这不是一个无缝的球体。",
          "de-DE":
            "Höhe, Rauheit, Mana, Auflösung, Bevölkerung und Ereignisse je aktivem Chunk. Die Koordinaten sind kartenblattgebunden: das ist kein nahtloser Globus.",
          "es-ES":
            "Elevación, rugosidad, maná, resolución, población y sucesos por bloque activo. Las coordenadas están referidas a su carta: esto no es un globo continuo.",
        },
      },
      {
        id: "surface-deltas",
        state: "bounded",
        domainMaturity: 3,
        observerMaturity: 4,
        area: "flux",
        title: {
          "en-US": "Material surface transitions",
          "ru-RU": "Переходы материальной поверхности",
          "zh-Hans": "物质表面转变",
          "de-DE": "Materielle Oberflächenübergänge",
          "es-ES": "Transiciones de superficie material",
        },
        detail: {
          "en-US":
            "A window of recent surface condition transitions with contact, mana-effect and local-mana traces. The window is bounded at 64 records.",
          "ru-RU":
            "Окно последних переходов состояния поверхности с трассами контакта, эффектов маны и локальной маны. Окно ограничено 64 записями.",
          "zh-Hans":
            "近期表面状态转变的窗口，附带接触迹线、魔力效应迹线与局部魔力迹线。该窗口上限为 64 条记录。",
          "de-DE":
            "Ein Fenster jüngster Übergänge des Oberflächenzustands mit Kontakt-, Mana-Effekt- und lokalen Mana-Spuren. Das Fenster ist auf 64 Einträge begrenzt.",
          "es-ES":
            "Una ventana de transiciones recientes del estado de la superficie con trazas de contacto, de efecto de maná y de maná local. La ventana está acotada a 64 registros.",
        },
      },
      {
        id: "gate-deltas",
        state: "bounded",
        domainMaturity: 3,
        observerMaturity: 4,
        area: "flux",
        title: {
          "en-US": "Local mana gates",
          "ru-RU": "Затворы локальной маны",
          "zh-Hans": "局部魔力闸门",
          "de-DE": "Lokale Mana-Schleusen",
          "es-ES": "Compuertas de maná local",
        },
        detail: {
          "en-US":
            "Only transitions into the closed state are projected. An empty list means no such transition occurred, not that gates are absent.",
          "ru-RU":
            "Наблюдатель получает только переходы в закрытое состояние. Пустой список означает отсутствие таких переходов, а не отсутствие затворов.",
          "zh-Hans":
            "只有转入关闭状态的转变会被投影。空列表意味着没有发生此类转变，而不是不存在闸门。",
          "de-DE":
            "Nur Übergänge in den geschlossenen Zustand werden projiziert. Eine leere Liste bedeutet, dass kein solcher Übergang stattfand, nicht dass es keine Schleusen gibt.",
          "es-ES":
            "Sólo se proyectan las transiciones hacia el estado cerrado. Una lista vacía significa que no ocurrió ninguna, no que no haya compuertas.",
        },
      },
      {
        id: "mana-field",
        state: "absent-projection",
        domainMaturity: 4,
        observerMaturity: 1,
        title: {
          "en-US": "Per-cell mana field",
          "ru-RU": "Поле маны по ячейкам",
          "zh-Hans": "逐单元格魔力场",
          "de-DE": "Mana-Feld je Zelle",
          "es-ES": "Campo de maná por celda",
        },
        detail: {
          "en-US":
            "The runtime holds a per-cell field; only chunk totals and the peak cell intensity are projected to the observer.",
          "ru-RU":
            "Среда хранит поле по ячейкам; наблюдателю проецируются только суммы по чанку и пиковая интенсивность.",
          "zh-Hans":
            "运行时持有逐单元格的场；投影给观测器的只有区块总量与峰值单元格强度。",
          "de-DE":
            "Die Laufzeitumgebung hält ein Feld je Zelle; dem Beobachter werden nur Chunk-Summen und die Spitzenintensität einer Zelle projiziert.",
          "es-ES":
            "El entorno de ejecución mantiene un campo por celda; al observador sólo se le proyectan los totales del bloque y la intensidad de la celda máxima.",
        },
      },
      {
        id: "resolution-field",
        state: "absent-projection",
        domainMaturity: 3,
        observerMaturity: 1,
        title: {
          "en-US": "Causal resolution field",
          "ru-RU": "Поле причинного разрешения",
          "zh-Hans": "因果分辨率场",
          "de-DE": "Feld der kausalen Auflösung",
          "es-ES": "Campo de resolución causal",
        },
        detail: {
          "en-US":
            "Relevance and level are available per chunk. The policy thresholds that separate levels are not projected, so the scale is shown in raw units.",
          "ru-RU":
            "По чанку доступны релевантность и уровень. Пороги переключения уровней политикой не проецируются, поэтому шкала показана в сырых единицах.",
          "zh-Hans":
            "每个区块都可获得相关度与层级。区分各层级的策略阈值不会被投影，因此刻度以原始单位显示。",
          "de-DE":
            "Relevanz und Stufe sind je Chunk verfügbar. Die Richtlinienschwellen zwischen den Stufen werden nicht projiziert, daher erscheint die Skala in Rohwerten.",
          "es-ES":
            "La relevancia y el nivel están disponibles por bloque. Los umbrales de política que separan los niveles no se proyectan, así que la escala se muestra en unidades brutas.",
        },
      },
    ],
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
    entries: [
      {
        id: "trace-anchors",
        state: "bounded",
        domainMaturity: 3,
        observerMaturity: 3,
        area: "flux",
        title: {
          "en-US": "Trace anchors",
          "ru-RU": "Якоря трасс",
          "zh-Hans": "迹线锚点",
          "de-DE": "Spuranker",
          "es-ES": "Anclas de traza",
        },
        detail: {
          "en-US":
            "Trace identifiers arrive with transitions and claims and tie an observation to its provenance. The traces themselves cannot be fetched.",
          "ru-RU":
            "Идентификаторы трасс приходят вместе с переходами и утверждениями и связывают наблюдаемое с провенансом. Сами трассы не запрашиваются.",
          "zh-Hans":
            "迹线标识符随转变与断言一同到达，把一次观测与其溯源绑定起来。迹线本身无法被取回。",
          "de-DE":
            "Spurkennungen treffen mit Übergängen und Aussagen ein und binden eine Beobachtung an ihre Herkunft. Die Spuren selbst sind nicht abrufbar.",
          "es-ES":
            "Los identificadores de traza llegan junto a transiciones y afirmaciones y atan una observación a su procedencia. Las trazas mismas no se pueden recuperar.",
        },
      },
      {
        id: "trace-graph",
        state: "absent-projection",
        domainMaturity: 3,
        observerMaturity: 0,
        title: {
          "en-US": "Provenance graph",
          "ru-RU": "Граф происхождения",
          "zh-Hans": "溯源图",
          "de-DE": "Provenienzgraph",
          "es-ES": "Grafo de procedencia",
        },
        detail: {
          "en-US":
            "CausalTraceStore holds ancestry and traversal, but no query kind, wire encoding or decoder exists for the graph. Only anchors are shown.",
          "ru-RU":
            "CausalTraceStore хранит предков и обход, но вида запроса, кодирования и декодера для графа нет. Пока показаны только якоря.",
          "zh-Hans":
            "CausalTraceStore 保有祖先关系与遍历能力，但该图既无查询类型，也无线路编码或解码器。目前只显示锚点。",
          "de-DE":
            "CausalTraceStore hält Ahnenketten und Traversierung, doch für den Graphen gibt es weder Abfrageart noch Drahtkodierung oder Dekoder. Gezeigt werden nur Anker.",
          "es-ES":
            "CausalTraceStore guarda la ascendencia y su recorrido, pero no existe tipo de consulta, codificación de transporte ni decodificador para el grafo. Sólo se muestran las anclas.",
        },
      },
      {
        id: "history",
        state: "absent-projection",
        domainMaturity: 2,
        observerMaturity: 0,
        title: {
          "en-US": "Historical state",
          "ru-RU": "Исторические состояния",
          "zh-Hans": "历史状态",
          "de-DE": "Historischer Zustand",
          "es-ES": "Estado histórico",
        },
        detail: {
          "en-US":
            "There are no queries against stored history. The series on these charts are assembled by the observer from received frames and are not authoritative.",
          "ru-RU":
            "Запросов к сохранённой истории нет. Ряд на графиках собран наблюдателем из полученных кадров и авторитетным не является.",
          "zh-Hans":
            "没有针对已存历史的查询。这些图表上的序列由观测器依据收到的帧汇集而成，不具权威性。",
          "de-DE":
            "Es gibt keine Abfragen gegen gespeicherte Historie. Die Reihen dieser Diagramme stellt der Beobachter aus empfangenen Bildern zusammen und sie sind nicht maßgeblich.",
          "es-ES":
            "No hay consultas contra la historia almacenada. Las series de estas gráficas las compone el observador a partir de los cuadros recibidos y no son autorizadas.",
        },
      },
    ],
  },
  {
    id: "explanation",
    title: {
      "en-US": "Explanation",
      "ru-RU": "Объяснение",
      "zh-Hans": "解释",
      "de-DE": "Erklärung",
      "es-ES": "Explicación",
    },
    entries: [
      {
        id: "explanation-ir",
        state: "bounded",
        domainMaturity: 3,
        observerMaturity: 4,
        area: "assay",
        title: {
          "en-US": "Typed Explanation IR",
          "ru-RU": "Типизированный Explanation IR",
          "zh-Hans": "带类型的 Explanation IR",
          "de-DE": "Typisiertes Explanation IR",
          "es-ES": "Explanation IR tipado",
        },
        detail: {
          "en-US":
            "Claims with a schema, value, confidence, evidence state and trace anchors. Bounded to the material-surface experiment.",
          "ru-RU":
            "Утверждения со схемой, значением, уверенностью, состоянием свидетельства и якорями трасс. Ограничен экспериментом материальной поверхности.",
          "zh-Hans":
            "带有模式、数值、置信度、证据状态与迹线锚点的断言。限于物质表面实验。",
          "de-DE":
            "Aussagen mit Schema, Wert, Konfidenz, Belegzustand und Spurankern. Begrenzt auf das Experiment zur materiellen Oberfläche.",
          "es-ES":
            "Afirmaciones con esquema, valor, confianza, estado de evidencia y anclas de traza. Acotado al experimento de superficie material.",
        },
      },
      {
        id: "rendered-explanation",
        state: "absent-projection",
        domainMaturity: 4,
        observerMaturity: 1,
        title: {
          "en-US": "Deterministic explanation text",
          "ru-RU": "Детерминированный текст объяснения",
          "zh-Hans": "确定性解释文本",
          "de-DE": "Deterministischer Erklärungstext",
          "es-ES": "Texto de explicación determinista",
        },
        detail: {
          "en-US":
            "The template renderer exists in Rust and is authoritative. The observer presents claim structure and does not reproduce the text on its own.",
          "ru-RU":
            "Шаблонный рендер существует в Rust и является авторитетным. Наблюдатель показывает структуру утверждений и не воспроизводит текст самостоятельно.",
          "zh-Hans":
            "模板渲染器位于 Rust 中并且是权威的。观测器只呈现断言结构，不自行复现该文本。",
          "de-DE":
            "Der Vorlagen-Renderer liegt in Rust und ist maßgeblich. Der Beobachter zeigt die Struktur der Aussagen und gibt den Text nicht selbst wieder.",
          "es-ES":
            "El renderizador de plantillas vive en Rust y es el autorizado. El observador presenta la estructura de las afirmaciones y no reproduce el texto por su cuenta.",
        },
      },
    ],
  },
  {
    id: "agents",
    title: {
      "en-US": "Agents and society",
      "ru-RU": "Агенты и общество",
      "zh-Hans": "智能体与社会",
      "de-DE": "Agenten und Gesellschaft",
      "es-ES": "Agentes y sociedad",
    },
    entries: [
      {
        id: "entities",
        state: "absent-projection",
        domainMaturity: 2,
        observerMaturity: 0,
        title: {
          "en-US": "Entities and actors",
          "ru-RU": "Сущности и акторы",
          "zh-Hans": "实体与行动者",
          "de-DE": "Wesenheiten und Akteure",
          "es-ES": "Entidades y actores",
        },
        detail: {
          "en-US":
            "Only the actor count and total population are available. The EntitySummary schema is defined, but no read model exists.",
          "ru-RU":
            "Доступно только число акторов и суммарное население. Схема EntitySummary описана, но модели чтения нет.",
          "zh-Hans":
            "只有行动者数量与人口总数可用。EntitySummary 模式已定义，但不存在读取模型。",
          "de-DE":
            "Verfügbar sind nur die Zahl der Akteure und die Gesamtbevölkerung. Das EntitySummary-Schema ist definiert, ein Lesemodell existiert nicht.",
          "es-ES":
            "Sólo están disponibles el recuento de actores y la población total. El esquema EntitySummary está definido, pero no existe modelo de lectura.",
        },
      },
      {
        id: "subjective",
        state: "absent-domain",
        domainMaturity: 1,
        observerMaturity: 0,
        title: {
          "en-US": "Subjective knowledge",
          "ru-RU": "Субъективное знание",
          "zh-Hans": "主观知识",
          "de-DE": "Subjektives Wissen",
          "es-ES": "Conocimiento subjetivo",
        },
        detail: {
          "en-US":
            "Everything shown here is an objective projection. Agent scene, memory and belief are at contract level and are not projected.",
          "ru-RU":
            "Всё показанное здесь — объективная проекция. Сцена, память и убеждения агентов находятся на уровне контрактов и не проецируются.",
          "zh-Hans":
            "此处显示的一切都是客观投影。智能体的场景、记忆与信念仍停留在契约层面，不会被投影。",
          "de-DE":
            "Alles hier Gezeigte ist eine objektive Projektion. Szene, Gedächtnis und Überzeugung der Agenten liegen auf Vertragsebene und werden nicht projiziert.",
          "es-ES":
            "Todo lo mostrado aquí es una proyección objetiva. La escena, la memoria y la creencia de los agentes están a nivel de contrato y no se proyectan.",
        },
      },
      {
        id: "language",
        state: "absent-domain",
        domainMaturity: 1,
        observerMaturity: 0,
        title: {
          "en-US": "Language",
          "ru-RU": "Язык",
          "zh-Hans": "语言",
          "de-DE": "Sprache",
          "es-ES": "Lengua",
        },
        detail: {
          "en-US":
            "Lexemes and subjective associations exist in the protocol, but the domain is not coupled to the runtime yet.",
          "ru-RU":
            "Лексемы и субъективные ассоциации описаны в протоколе, но домен ещё не связан со средой.",
          "zh-Hans":
            "协议中已有词位与主观联想，但该领域尚未与运行时耦合。",
          "de-DE":
            "Lexeme und subjektive Assoziationen existieren im Protokoll, doch die Domäne ist noch nicht an die Laufzeitumgebung gekoppelt.",
          "es-ES":
            "Los lexemas y las asociaciones subjetivas existen en el protocolo, pero el dominio todavía no está acoplado al entorno de ejecución.",
        },
      },
      {
        id: "social",
        state: "absent-domain",
        domainMaturity: 1,
        observerMaturity: 0,
        title: {
          "en-US": "Social structure",
          "ru-RU": "Социальная структура",
          "zh-Hans": "社会结构",
          "de-DE": "Sozialstruktur",
          "es-ES": "Estructura social",
        },
        detail: {
          "en-US": "Contracts exist; agent-inferred structure and a read model do not.",
          "ru-RU": "Контракты существуют; выводимой агентами структуры и модели чтения — нет.",
          "zh-Hans": "契约已经存在；由智能体推断出的结构与读取模型则没有。",
          "de-DE":
            "Verträge existieren; von Agenten erschlossene Struktur und ein Lesemodell nicht.",
          "es-ES":
            "Los contratos existen; la estructura inferida por los agentes y un modelo de lectura, no.",
        },
      },
      {
        id: "practices",
        state: "absent-domain",
        domainMaturity: 1,
        observerMaturity: 0,
        title: {
          "en-US": "Practices and concepts",
          "ru-RU": "Практики и концепты",
          "zh-Hans": "实践与概念",
          "de-DE": "Praktiken und Konzepte",
          "es-ES": "Prácticas y conceptos",
        },
        detail: {
          "en-US":
            "Practice transmission and concept evolution require embodied execution that does not exist yet.",
          "ru-RU":
            "Передача практик и эволюция концептов требуют исполняемого воплощения, которого пока нет.",
          "zh-Hans": "实践的传递与概念的演化需要具身执行，而这尚不存在。",
          "de-DE":
            "Weitergabe von Praktiken und Konzeptentwicklung erfordern verkörperte Ausführung, die es noch nicht gibt.",
          "es-ES":
            "La transmisión de prácticas y la evolución de conceptos requieren una ejecución encarnada que todavía no existe.",
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
