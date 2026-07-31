/**
 * Presentation models.
 *
 * The decoded protocol structures are the authority; the shapes below exist only so views
 * can be written against stable, already-reduced data. Nothing here invents a value: every
 * field is either copied from an observer payload or derived from several of them by an
 * operation that is named after what it does.
 *
 * Derived series carry `derived: true` so surfaces can label them as observer-side and not
 * as simulation state (INV-022).
 */

import type {
  HydrologyCellDelta,
  HydrologySummary,
  HydrologyTransferSummary,
  MaterialSurfaceDelta,
  MaterialSurfaceGateDelta,
  RuntimeSummary,
  SpatialChunkSummary,
  WorldChunkSnapshot,
} from "@causafera/observer-protocol";
import { HYDROLOGY_SUMMARY_SCHEMA_ABSENT } from "@causafera/observer-protocol";

/** Each signal is one measured quantity with one reserved hue. */
export type SignalId =
  | "mana"
  | "trace"
  | "resolution"
  | "life"
  | "physical"
  | "refused"
  | "water";

export const SIGNAL_VARIABLE: Record<SignalId, string> = {
  mana: "var(--sig-mana)",
  trace: "var(--sig-trace)",
  resolution: "var(--sig-resolution)",
  life: "var(--sig-life)",
  physical: "var(--sig-physical)",
  refused: "var(--sig-refused)",
  water: "var(--sig-water)",
};

export const SIGNAL_WASH: Record<SignalId, string> = {
  mana: "var(--sig-mana-wash)",
  trace: "var(--sig-trace-wash)",
  resolution: "var(--sig-resolution-wash)",
  life: "var(--sig-life-wash)",
  physical: "var(--sig-physical-wash)",
  refused: "var(--sig-refused-wash)",
  water: "var(--sig-water-wash)",
};

/* ---------------------------------------------------------------- series -- */

export interface SeriesPoint {
  ticks: number;
  value: number;
}

export interface Series {
  id: string;
  signal: SignalId;
  points: SeriesPoint[];
  /** True when the values are differences the observer computed between received frames. */
  derived: boolean;
  dashed?: boolean;
}

/** Cumulative counters read straight from consecutive runtime summaries. */
export function levelSeries(
  history: readonly RuntimeSummary[],
  id: string,
  signal: SignalId,
  read: (summary: RuntimeSummary) => bigint,
): Series {
  return {
    id,
    signal,
    derived: false,
    points: history.map((summary) => ({
      ticks: Number(summary.simulationTicks),
      value: Number(read(summary)),
    })),
  };
}

/**
 * Per-tick rate between received frames.
 *
 * The observer only sees the frames it asked for, so a rate is an average across the gap
 * between two summaries, not an instantaneous simulation quantity.
 */
export function rateSeries(
  history: readonly RuntimeSummary[],
  id: string,
  signal: SignalId,
  read: (summary: RuntimeSummary) => bigint,
): Series {
  const points: SeriesPoint[] = [];
  for (let index = 1; index < history.length; index += 1) {
    const current = history[index]!;
    const earlier = history[index - 1]!;
    const span = Number(current.simulationTicks - earlier.simulationTicks);
    if (span <= 0) continue;
    points.push({
      ticks: Number(current.simulationTicks),
      value: Number(read(current) - read(earlier)) / span,
    });
  }
  return { id, signal, derived: true, points };
}

/* ----------------------------------------------------------------- atlas -- */

export interface ChunkRecord {
  key: string;
  chartId: bigint;
  chunkX: number;
  chunkY: number;
  chunkZ: number;
  minimumElevationMm: number;
  maximumElevationMm: number;
  meanRoughnessMm: number;
  manaTotal: number;
  resolutionRelevance: number;
  resolutionLevel: number;
  populationTotal: number;
  causalEventCount: number;
  latestTraceId: bigint;
  /** Material-surface transitions observed in this chunk within the bounded delta window. */
  transitions: number;
}

export interface ChartRecord {
  chartId: bigint;
  chunks: ChunkRecord[];
}

export interface Atlas {
  charts: ChartRecord[];
  chunks: ChunkRecord[];
  elevationMin: number;
  elevationMax: number;
  manaMax: number;
  populationMax: number;
  eventMax: number;
}

export function chunkKey(chunk: {
  chartId: bigint;
  chunkX: number;
  chunkY: number;
  chunkZ: number;
}): string {
  return `${chunk.chartId}:${chunk.chunkX}:${chunk.chunkY}:${chunk.chunkZ}`;
}

export function buildAtlas(world: WorldChunkSnapshot | undefined): Atlas {
  const empty: Atlas = {
    charts: [],
    chunks: [],
    elevationMin: 0,
    elevationMax: 0,
    manaMax: 0,
    populationMax: 0,
    eventMax: 0,
  };
  if (world === undefined || world.chunks.length === 0) return empty;

  const transitions = new Map<string, number>();
  for (const delta of world.materialSurfaceDeltas) {
    const key = chunkKey(delta);
    transitions.set(key, (transitions.get(key) ?? 0) + 1);
  }

  const chunks = world.chunks
    .map<ChunkRecord>((chunk: SpatialChunkSummary) => ({
      key: chunkKey(chunk),
      chartId: chunk.chartId,
      chunkX: chunk.chunkX,
      chunkY: chunk.chunkY,
      chunkZ: chunk.chunkZ,
      minimumElevationMm: chunk.minimumElevationMm,
      maximumElevationMm: chunk.maximumElevationMm,
      meanRoughnessMm: chunk.meanRoughnessMm,
      manaTotal: Number(chunk.manaTotal),
      resolutionRelevance: Number(chunk.resolutionRelevance),
      resolutionLevel: chunk.resolutionLevel,
      populationTotal: Number(chunk.populationTotal),
      causalEventCount: Number(chunk.causalEventCount),
      latestTraceId: chunk.latestTraceId,
      transitions: transitions.get(chunkKey(chunk)) ?? 0,
    }))
    .sort(
      (a, b) =>
        Number(a.chartId - b.chartId) || a.chunkZ - b.chunkZ || a.chunkY - b.chunkY || a.chunkX - b.chunkX,
    );

  const charts: ChartRecord[] = [];
  for (const chunk of chunks) {
    const chart = charts.find((entry) => entry.chartId === chunk.chartId);
    if (chart === undefined) charts.push({ chartId: chunk.chartId, chunks: [chunk] });
    else chart.chunks.push(chunk);
  }

  return {
    charts,
    chunks,
    elevationMin: Math.min(...chunks.map((chunk) => chunk.minimumElevationMm)),
    elevationMax: Math.max(...chunks.map((chunk) => chunk.maximumElevationMm)),
    manaMax: Math.max(1, ...chunks.map((chunk) => chunk.manaTotal)),
    populationMax: Math.max(1, ...chunks.map((chunk) => chunk.populationTotal)),
    eventMax: Math.max(1, ...chunks.map((chunk) => chunk.causalEventCount)),
  };
}

/* -------------------------------------------------------- surface ladder -- */

/**
 * One tracked material surface, addressed by chart-qualified chunk plus the cell ordinal
 * inside that chunk. A step is one observed condition transition with its provenance.
 */
export interface SurfaceStep {
  tick: number;
  beforeCondition: number;
  afterCondition: number;
  manaTotal: number;
  contactTraceId?: bigint;
  manaEffectTraceId?: bigint;
  manaTransitionTraceId?: bigint;
  manaBefore?: number;
  manaAfter?: number;
  localManaBefore?: number;
  localManaAfter?: number;
  localManaTransitionTraceId?: bigint;
}

export interface SurfaceLadder {
  key: string;
  chartId: bigint;
  chunkX: number;
  chunkY: number;
  chunkZ: number;
  cellOrdinal: number;
  /** Cell ordinal decoded to its position inside the chunk lattice. */
  cell: { x: number; y: number; z: number };
  steps: SurfaceStep[];
  firstTick: number;
  lastTick: number;
  minCondition: number;
  maxCondition: number;
  manaEffects: number;
  localManaEvents: number;
}

/** Chunks are CHUNK_SIZE³ lattices; the ordinal is the dense flat index z·s² + y·s + x. */
export const CHUNK_SIZE = 32;

export function decodeCellOrdinal(ordinal: number): { x: number; y: number; z: number } {
  const x = ordinal % CHUNK_SIZE;
  const y = Math.floor(ordinal / CHUNK_SIZE) % CHUNK_SIZE;
  const z = Math.floor(ordinal / (CHUNK_SIZE * CHUNK_SIZE));
  return { x, y, z };
}

export function surfaceKey(delta: {
  chartId: bigint;
  chunkX: number;
  chunkY: number;
  chunkZ: number;
  cellOrdinal: number;
}): string {
  return `${chunkKey(delta)}#${delta.cellOrdinal}`;
}

export function buildLadders(deltas: readonly MaterialSurfaceDelta[]): SurfaceLadder[] {
  const byKey = new Map<string, SurfaceLadder>();
  for (const delta of deltas) {
    const key = surfaceKey(delta);
    let ladder = byKey.get(key);
    if (ladder === undefined) {
      ladder = {
        key,
        chartId: delta.chartId,
        chunkX: delta.chunkX,
        chunkY: delta.chunkY,
        chunkZ: delta.chunkZ,
        cellOrdinal: delta.cellOrdinal,
        cell: decodeCellOrdinal(delta.cellOrdinal),
        steps: [],
        firstTick: Number.POSITIVE_INFINITY,
        lastTick: Number.NEGATIVE_INFINITY,
        minCondition: Number.POSITIVE_INFINITY,
        maxCondition: Number.NEGATIVE_INFINITY,
        manaEffects: 0,
        localManaEvents: 0,
      };
      byKey.set(key, ladder);
    }
    const step: SurfaceStep = {
      tick: Number(delta.transitionTick),
      beforeCondition: Number(delta.beforeCondition),
      afterCondition: Number(delta.afterCondition),
      manaTotal: Number(delta.manaTotal),
      contactTraceId: delta.contactTraceId,
      manaEffectTraceId: delta.manaEffectTraceId,
      manaTransitionTraceId: delta.manaTransitionTraceId,
      manaBefore: optionalNumber(delta.manaBefore),
      manaAfter: optionalNumber(delta.manaAfter),
      localManaBefore: optionalNumber(delta.localManaBefore),
      localManaAfter: optionalNumber(delta.localManaAfter),
      localManaTransitionTraceId: delta.localManaTransitionTraceId,
    };
    ladder.steps.push(step);
    ladder.firstTick = Math.min(ladder.firstTick, step.tick);
    ladder.lastTick = Math.max(ladder.lastTick, step.tick);
    ladder.minCondition = Math.min(ladder.minCondition, step.beforeCondition);
    ladder.maxCondition = Math.max(ladder.maxCondition, step.afterCondition);
    if (step.manaEffectTraceId !== undefined) ladder.manaEffects += 1;
    if (step.localManaAfter !== undefined) ladder.localManaEvents += 1;
  }
  const ladders = [...byKey.values()];
  for (const ladder of ladders) ladder.steps.sort((a, b) => a.tick - b.tick);
  return ladders.sort(
    (a, b) => Number(a.chartId - b.chartId) || a.chunkX - b.chunkX || a.cellOrdinal - b.cellOrdinal,
  );
}

export interface LadderExtent {
  tickMin: number;
  tickMax: number;
  conditionMin: number;
  conditionMax: number;
}

export function ladderExtent(ladders: readonly SurfaceLadder[]): LadderExtent {
  if (ladders.length === 0) return { tickMin: 0, tickMax: 1, conditionMin: 0, conditionMax: 1 };
  const tickMin = Math.min(...ladders.map((ladder) => ladder.firstTick));
  const tickMax = Math.max(...ladders.map((ladder) => ladder.lastTick));
  const conditionMin = Math.min(...ladders.map((ladder) => ladder.minCondition));
  const conditionMax = Math.max(...ladders.map((ladder) => ladder.maxCondition));
  return {
    tickMin,
    tickMax: tickMax === tickMin ? tickMin + 1 : tickMax,
    conditionMin,
    conditionMax: conditionMax === conditionMin ? conditionMin + 1 : conditionMax,
  };
}

/* ------------------------------------------------------------ hydrology -- */

/**
 * Whether the payload carries water at all.
 *
 * Three states, and conflating any two of them would misreport the world. A
 * payload written before hydrology existed says nothing about water; a build
 * that has hydrology writes the group even when the domain is off, and that is
 * a session with no water rather than a session that did not say; and an
 * enabled session with no resident field is neither.
 */
export type WaterPresence = "unreported" | "disabled" | "observed";

export function waterPresence(summary: RuntimeSummary | undefined): WaterPresence {
  if (summary === undefined) return "unreported";
  if (summary.hydrology.schemaVersion === HYDROLOGY_SUMMARY_SCHEMA_ABSENT) return "unreported";
  return summary.hydrology.activeChunkCount === 0 ? "disabled" : "observed";
}

/** Every storage the ledger closes over, summed. Cell storages plus conveyance. */
export function totalWater(hydrology: HydrologySummary): bigint {
  return (
    hydrology.totalSurface +
    hydrology.totalSoil +
    hydrology.totalGroundwater +
    hydrology.totalConveyance
  );
}

/**
 * One cell's committed storage change, reduced to what a reader compares.
 *
 * The protocol sends before and after for all three storages; the differences
 * are computed here rather than in a view so that every surface showing this
 * cell shows the same subtraction.
 */
export interface WaterCell {
  key: string;
  chunkKey: string;
  chartId: bigint;
  chunkX: number;
  chunkY: number;
  chunkZ: number;
  cellOrdinal: number;
  cell: { x: number; y: number; z: number };
  tick: number;
  surfaceChange: bigint;
  soilChange: bigint;
  groundwaterChange: bigint;
  netForcing: bigint;
  netLateralFlow: bigint;
  /**
   * The most recent of the cell's three bucket anchors: the event that last
   * settled any part of this transition. The batch anchor that closed the
   * ledger for the whole tick is the same for every row of a tick, so it is
   * reported once in the Observatory's ledger rather than repeated here.
   */
  transitionTraceId: bigint;
}

export function buildWaterCells(deltas: readonly HydrologyCellDelta[]): WaterCell[] {
  return deltas
    .map<WaterCell>((delta) => {
      const surfaceChange = delta.surfaceAfter - delta.surfaceBefore;
      const soilChange = delta.soilAfter - delta.soilBefore;
      const groundwaterChange = delta.groundwaterAfter - delta.groundwaterBefore;
      return {
        key: `${surfaceKey(delta)}@${delta.transitionTick}`,
        chunkKey: chunkKey(delta),
        chartId: delta.chartId,
        chunkX: delta.chunkX,
        chunkY: delta.chunkY,
        chunkZ: delta.chunkZ,
        cellOrdinal: delta.cellOrdinal,
        cell: decodeHydrologyCellOrdinal(delta.cellOrdinal),
        tick: Number(delta.transitionTick),
        surfaceChange,
        soilChange,
        groundwaterChange,
        netForcing: delta.netForcing,
        netLateralFlow: delta.netLateralFlow,
        transitionTraceId: delta.transitionTraceId,
      };
    })
    .sort((a, b) => b.tick - a.tick || a.cellOrdinal - b.cellOrdinal);
}

/**
 * The hydrology lattice is one cell deep and 32 x 32 across a chunk, unlike the
 * 32³ lattice a material surface ordinal addresses. Decoding it with the cubic
 * rule would place every cell of the plan at z 0 and x, y wrong past the first
 * row, so it gets its own decoding rather than sharing one that happens to
 * agree for the first 32 ordinals.
 */
export function decodeHydrologyCellOrdinal(ordinal: number): { x: number; y: number; z: number } {
  return { x: ordinal % CHUNK_SIZE, y: Math.floor(ordinal / CHUNK_SIZE), z: 0 };
}

/**
 * One accepted, partly accepted or wholly rejected movement, with the limiter
 * evidence made comparable.
 *
 * `acceptance` is accepted over requested. A limiter that engaged is the whole
 * reason all three volumes travel: a movement that asked for more than it got is
 * evidence of a bound doing its work, and a table that showed only what moved
 * would make it indistinguishable from a process that had nothing to do.
 */
export interface WaterTransfer {
  key: string;
  processKind: number;
  tick: number;
  requested: bigint;
  accepted: bigint;
  unaccepted: bigint;
  /** 0..1, or undefined when nothing was requested and the ratio has no meaning. */
  acceptance?: number;
  limited: boolean;
  transferTraceId: bigint;
  forcingOriginTraceId: bigint | null;
}

export function buildWaterTransfers(
  summaries: readonly HydrologyTransferSummary[],
): WaterTransfer[] {
  return summaries
    .map<WaterTransfer>((transfer, index) => ({
      key: `${transfer.transferTraceId}:${index}`,
      processKind: transfer.processKind,
      tick: Number(transfer.tick),
      requested: transfer.requestedVolume,
      accepted: transfer.acceptedVolume,
      unaccepted: transfer.unacceptedVolume,
      acceptance:
        transfer.requestedVolume === 0n
          ? undefined
          : Number(transfer.acceptedVolume) / Number(transfer.requestedVolume),
      limited: transfer.unacceptedVolume > 0n,
      transferTraceId: transfer.transferTraceId,
      forcingOriginTraceId: transfer.forcingOriginTraceId,
    }))
    .sort((a, b) => b.tick - a.tick || Number(b.accepted - a.accepted));
}

/* ----------------------------------------------------------------- gates -- */

export interface GateRecord {
  key: string;
  chartId: bigint;
  chunkX: number;
  chunkY: number;
  chunkZ: number;
  cellOrdinal: number;
  tick: number;
  beforeActive: boolean;
  afterActive: boolean;
  localManaBefore: number;
  localManaAfter: number;
  localManaTransitionTraceId: bigint;
  gateTransitionTraceId: bigint;
  contactTraceId?: bigint;
}

export function buildGates(gates: readonly MaterialSurfaceGateDelta[]): GateRecord[] {
  return gates
    .map<GateRecord>((gate) => ({
      key: `${surfaceKey(gate)}@${gate.transitionTick}`,
      chartId: gate.chartId,
      chunkX: gate.chunkX,
      chunkY: gate.chunkY,
      chunkZ: gate.chunkZ,
      cellOrdinal: gate.cellOrdinal,
      tick: Number(gate.transitionTick),
      beforeActive: gate.beforeActive,
      afterActive: gate.afterActive,
      localManaBefore: Number(gate.localManaBefore),
      localManaAfter: Number(gate.localManaAfter),
      localManaTransitionTraceId: gate.localManaTransitionTraceId,
      gateTransitionTraceId: gate.gateTransitionTraceId,
      contactTraceId: gate.contactTraceId,
    }))
    .sort((a, b) => b.tick - a.tick);
}

/* ------------------------------------------------------------- ledger ---- */

export type TraceRole = "contact" | "manaEffect" | "manaTransition" | "localMana";

export interface LedgerEntry {
  id: string;
  tick: number;
  surface: string;
  chunkX: number;
  chunkY: number;
  chunkZ: number;
  chartId: bigint;
  cellOrdinal: number;
  beforeCondition: number;
  afterCondition: number;
  manaTotal: number;
  traces: { id: bigint; roles: TraceRole[] }[];
  localManaBefore?: number;
  localManaAfter?: number;
}

export function buildLedger(deltas: readonly MaterialSurfaceDelta[]): LedgerEntry[] {
  return deltas
    .map<LedgerEntry>((delta, index) => {
      // One trace can anchor several roles — a mana transition and its local-mana coupling
      // are frequently the same committed event. Collapse to one chip carrying every role.
      const byId = new Map<bigint, LedgerEntry["traces"][number]>();
      const anchor = (role: LedgerEntry["traces"][number]["roles"][number], id?: bigint) => {
        if (id === undefined) return;
        const existing = byId.get(id);
        if (existing === undefined) byId.set(id, { id, roles: [role] });
        else if (!existing.roles.includes(role)) existing.roles.push(role);
      };
      anchor("contact", delta.contactTraceId);
      anchor("manaEffect", delta.manaEffectTraceId);
      anchor("manaTransition", delta.manaTransitionTraceId);
      anchor("localMana", delta.localManaTransitionTraceId);
      const traces = [...byId.values()];
      return {
        id: `${surfaceKey(delta)}@${delta.transitionTick}:${index}`,
        tick: Number(delta.transitionTick),
        surface: surfaceKey(delta),
        chartId: delta.chartId,
        chunkX: delta.chunkX,
        chunkY: delta.chunkY,
        chunkZ: delta.chunkZ,
        cellOrdinal: delta.cellOrdinal,
        beforeCondition: Number(delta.beforeCondition),
        afterCondition: Number(delta.afterCondition),
        manaTotal: Number(delta.manaTotal),
        traces,
        localManaBefore: optionalNumber(delta.localManaBefore),
        localManaAfter: optionalNumber(delta.localManaAfter),
      };
    })
    .sort((a, b) => b.tick - a.tick || Number(b.chartId - a.chartId));
}

/* ------------------------------------------------------------ summaries -- */

export interface ActivityRate {
  id: string;
  signal: SignalId;
  total: bigint;
  perTick: number;
  available: boolean;
}

/** Difference between the two most recent received summaries, normalised per tick. */
export function activityRates(history: readonly RuntimeSummary[]): Map<string, ActivityRate> {
  const rates = new Map<string, ActivityRate>();
  const current = history[history.length - 1];
  const earlier = history[history.length - 2];
  const measures: { id: string; signal: SignalId; read: (s: RuntimeSummary) => bigint }[] = [
    { id: "traces", signal: "trace", read: (s) => s.causalTraceCount },
    { id: "physicalEvents", signal: "physical", read: (s) => s.physicalEvents },
    { id: "manaCellChanges", signal: "mana", read: (s) => s.manaCellChanges },
    { id: "manaPhysicalEffects", signal: "mana", read: (s) => s.manaPhysicalEffects },
    { id: "resolutionTransitions", signal: "resolution", read: (s) => s.resolutionTransitions },
    { id: "actorActionsCommitted", signal: "life", read: (s) => s.actorActionsCommitted },
    { id: "actorActionsRejected", signal: "refused", read: (s) => s.actorActionsRejected },
    { id: "populationMovements", signal: "life", read: (s) => s.populationMovements },
    { id: "populationBirths", signal: "life", read: (s) => s.populationBirths },
    { id: "populationDeaths", signal: "refused", read: (s) => s.populationDeaths },
  ];
  for (const measure of measures) {
    const total = current === undefined ? 0n : measure.read(current);
    let perTick = 0;
    let available = false;
    if (current !== undefined && earlier !== undefined) {
      const span = Number(current.simulationTicks - earlier.simulationTicks);
      if (span > 0) {
        perTick = Number(measure.read(current) - measure.read(earlier)) / span;
        available = true;
      }
    }
    rates.set(measure.id, { id: measure.id, signal: measure.signal, total, perTick, available });
  }
  return rates;
}

/** Admission ratio of actor actions. Both terms come from the same summary. */
export function admissionRatio(summary: RuntimeSummary | undefined): number | undefined {
  if (summary === undefined) return undefined;
  const attempted = summary.actorActionsCommitted + summary.actorActionsRejected;
  if (attempted === 0n) return undefined;
  return Number(summary.actorActionsCommitted) / Number(attempted);
}

function optionalNumber(value: bigint | undefined): number | undefined {
  return value === undefined ? undefined : Number(value);
}
