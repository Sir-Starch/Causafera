/**
 * Observer session controller.
 *
 * Owns the transport, the run loop, and the bounded observer-side history. It lives outside
 * React so that connection lifecycle is not tied to component mounting, and it publishes
 * state through `Store` so panels subscribe to slices rather than to everything.
 *
 * Two boundaries are load-bearing:
 *
 * - `history` is an observer-side FIFO of received summaries. It is presentation state, not
 *   simulation state, and every surface that reads it must say so.
 * - `world` is only polled while a mounted panel demands the feed.
 */

import { FieldRasterKind } from "@causafera/observer-protocol";
import type {
  ExplanationReport,
  FieldRaster,
  SpatialChunkSummary,
  WorldChunkSnapshot,
} from "@causafera/observer-protocol";
import type { RuntimeSummary } from "@causafera/observer-protocol";

import { initialLocale, rememberLocale } from "../i18n";
import { DemandRegistry, Store } from "./store";
import {
  ObserverClient,
  hasTauriTransport,
  tauriChannel,
  type ByteChannel,
  type Exchange,
} from "./transport";
import type { ObserverLocale } from "./format";

export type ConnectionState = "idle" | "connecting" | "connected" | "unavailable" | "error";
export type BatchSize = 1 | 4 | 16 | 64;
export type Feed = "world";

export const BATCH_SIZES: readonly BatchSize[] = [1, 4, 16, 64];

/** Bounded observer-side buffers. The observer must never grow an unbounded queue. */
export const HISTORY_CAPACITY = 256;
export const EXCHANGE_CAPACITY = 120;
/**
 * Decoded rasters held at once.
 *
 * The active set is nine chunks, and a session with water carries five fields
 * per chunk: terrain, mana, and the three water buckets. The capacity has to
 * clear that product with room to spare, because evicting a lattice the current
 * frame is still drawing would make the map flicker between what it has and
 * what it just threw away.
 */
export const RASTER_CAPACITY = 96;

/**
 * The water buckets a hydrology-enabled session fetches per chunk.
 *
 * All three rather than only the mounted lens's: they are the same quantity in
 * three places, switching between them must not stall on a round trip, and soil
 * and groundwater hold water in worlds whose surface is dry.
 */
const HYDROLOGY_FIELDS: readonly FieldRasterKind[] = [
  FieldRasterKind.HydrologySurfaceWater,
  FieldRasterKind.HydrologySoilWater,
  FieldRasterKind.HydrologyGroundwater,
];

/** Per-chunk, per-field identity of a decoded raster. */
export function rasterKey(
  field: FieldRasterKind,
  chunk: { chartId: bigint; chunkX: number; chunkY: number; chunkZ: number },
): string {
  return `${field}|${chunk.chartId}:${chunk.chunkX}:${chunk.chunkY}:${chunk.chunkZ}`;
}

const RUN_INTERVAL_MS = 340;

export interface Negotiation {
  protocolVersion: number;
  capabilities: number[];
  timeAtConnect: bigint;
}

export interface SessionState {
  connection: ConnectionState;
  transportLabel: string;
  /** True when the byte channel replays a recorded capture rather than a live runtime. */
  replaying: boolean;
  negotiation?: Negotiation;
  summary?: RuntimeSummary;
  previous?: RuntimeSummary;
  world?: WorldChunkSnapshot;
  worldTicks?: bigint;
  /** Decoded per-cell lattices, keyed by `rasterKey`. Empty until one arrives. */
  rasters: ReadonlyMap<string, FieldRaster>;
  explanation?: ExplanationReport;
  explanationTicks?: bigint;
  history: RuntimeSummary[];
  exchanges: Exchange[];
  error?: string;
  /** Set when the failure is the export cap, which is a limit rather than a fault. */
  errorKind?: "export-cap";
  running: boolean;
  analyzing: boolean;
  batch: BatchSize;
  seed: number;
  locale: ObserverLocale;
}

const initialState: SessionState = {
  connection: "idle",
  transportLabel: "none",
  replaying: false,
  rasters: new Map(),
  history: [],
  exchanges: [],
  running: false,
  analyzing: false,
  batch: 4,
  seed: 0,
  // A remembered choice, else the browser's language preferences, else English.
  locale: initialLocale(),
};

export class ObserverSessionController {
  readonly store = new Store<SessionState>(initialState);
  readonly demand = new DemandRegistry<Feed>();

  private client?: ObserverClient;
  private started = false;
  private timer?: number;
  private generation = 0;
  private readonly rasters = new Map<string, FieldRaster>();

  constructor(private readonly resolveChannel: () => ByteChannel | undefined) {
    this.demand.subscribe(() => {
      if (this.demand.demanded("world") && this.store.snapshot.connection === "connected") {
        void this.refreshWorld();
      }
    });
  }

  /** Idempotent: React StrictMode mounts twice in development. */
  start(): void {
    if (this.started) return;
    this.started = true;
    const channel = this.resolveChannel();
    if (channel === undefined) {
      this.store.patch({ connection: "unavailable", transportLabel: "none" });
      return;
    }
    this.client = new ObserverClient(channel, (exchange) => this.recordExchange(exchange));
    this.store.patch({
      transportLabel: channel.label,
      replaying: channel.label !== tauriChannel.label,
    });
    void this.connect();
  }

  async connect(): Promise<void> {
    const client = this.client;
    if (client === undefined) return;
    const generation = (this.generation += 1);
    this.store.patch({ connection: "connecting", error: undefined, errorKind: undefined });
    this.forgetRasters();
    try {
      const response = await client.connect(this.store.snapshot.locale);
      if (generation !== this.generation) return;
      const update = await client.openRuntimeStream();
      if (generation !== this.generation) return;
      this.store.patch({
        connection: "connected",
        negotiation: {
          protocolVersion: response.selectedProtocolVersion,
          capabilities: response.capabilities,
          timeAtConnect: response.currentTime,
        },
        summary: update.summary,
        previous: undefined,
        history: [update.summary],
        explanation: undefined,
        explanationTicks: undefined,
      });
      if (this.demand.demanded("world")) await this.refreshWorld();
    } catch (cause) {
      if (generation !== this.generation) return;
      this.store.patch({
        connection: "error",
        error: message(cause),
        errorKind: errorKindOf(cause),
        running: false,
      });
    }
  }

  setLocale(locale: ObserverLocale): void {
    if (this.store.snapshot.locale === locale) return;
    this.store.patch({ locale });
    rememberLocale(locale);
    // Renegotiating carries the observer locale to the protocol handler. It cannot change
    // any state hash (INV-007); it only tells the session which locale is observing.
    void this.client?.connect(locale).catch(() => {});
  }

  setBatch(batch: BatchSize): void {
    this.store.patch({ batch });
  }

  setSeed(seed: number): void {
    this.store.patch({ seed });
  }

  toggleRun(): void {
    if (this.store.snapshot.connection !== "connected") return;
    if (this.store.snapshot.running) {
      this.stop();
      return;
    }
    this.store.patch({ running: true, error: undefined, errorKind: undefined });
    this.scheduleTick(0);
  }

  stop(): void {
    if (this.timer !== undefined) {
      window.clearTimeout(this.timer);
      this.timer = undefined;
    }
    if (this.store.snapshot.running) this.store.patch({ running: false });
  }

  async step(): Promise<void> {
    this.stop();
    await this.advance(this.store.snapshot.batch);
  }

  async reset(): Promise<void> {
    const client = this.client;
    if (client === undefined || this.store.snapshot.connection !== "connected") return;
    this.stop();
    const generation = (this.generation += 1);
    // A new seed is a new world: every lattice held from the old one is stale.
    this.forgetRasters();
    try {
      const update = await client.reset(this.store.snapshot.seed);
      if (generation !== this.generation) return;
      this.store.patch({
        summary: update.summary,
        previous: undefined,
        history: [update.summary],
        explanation: undefined,
        explanationTicks: undefined,
        world: undefined,
        worldTicks: undefined,
        error: undefined,
        errorKind: undefined,
      });
      if (this.demand.demanded("world")) await this.refreshWorld();
    } catch (cause) {
      this.store.patch({ error: message(cause), errorKind: errorKindOf(cause) });
    }
  }

  async analyze(): Promise<void> {
    const client = this.client;
    if (client === undefined || this.store.snapshot.connection !== "connected") return;
    if (this.store.snapshot.analyzing) return;
    this.stop();
    this.store.patch({ analyzing: true, error: undefined, errorKind: undefined });
    try {
      const report = await client.analyze();
      this.store.patch({
        explanation: report,
        explanationTicks: this.store.snapshot.summary?.simulationTicks,
      });
    } catch (cause) {
      this.store.patch({ error: message(cause), errorKind: errorKindOf(cause) });
    } finally {
      this.store.patch({ analyzing: false });
    }
  }

  async refreshWorld(): Promise<void> {
    const client = this.client;
    if (client === undefined || this.store.snapshot.connection !== "connected") return;
    try {
      const world = await client.world();
      this.store.patch({ world, worldTicks: world.simulationTicks });
      await this.refreshRasters(world);
    } catch (cause) {
      this.store.patch({ error: message(cause), errorKind: errorKindOf(cause) });
    }
  }

  /**
   * Fetch the per-cell lattices for the chunks the observer has received.
   *
   * Terrain is generated once and does not change per tick, so it is fetched
   * once per chunk and held against its generation trace. The mana field does
   * change, so it is refetched with every world frame — one bounded request per
   * chunk, never a chart dump.
   *
   * The water buckets are fetched on the same terms, and only when the summary
   * says the session holds water. A world with hydrology disabled would
   * otherwise pay twenty-seven refusals a frame to be told what its own summary
   * already said.
   */
  private async refreshRasters(world: WorldChunkSnapshot): Promise<void> {
    const client = this.client;
    if (client === undefined) return;
    const wet = (this.store.snapshot.summary?.hydrology.activeChunkCount ?? 0) > 0;
    // Issued together rather than one after another. The session serialises them
    // on its own lock either way, but a frame of nine chunks otherwise pays the
    // bridge's per-call overhead nine times end to end instead of once.
    const wanted = world.chunks.flatMap((chunk) => {
      const fields = [FieldRasterKind.ManaIntensity];
      // Terrain is generated once and does not change per tick.
      if (!this.rasters.has(rasterKey(FieldRasterKind.TerrainElevation, chunk))) {
        fields.push(FieldRasterKind.TerrainElevation);
      }
      if (wet) fields.push(...HYDROLOGY_FIELDS);
      return fields.map((field) => this.fetchRaster(client, field, chunk));
    });
    const changed = (await Promise.all(wanted)).some(Boolean);
    if (changed) this.store.patch({ rasters: new Map(this.rasters) });
  }

  private async fetchRaster(
    client: ObserverClient,
    field: FieldRasterKind,
    chunk: SpatialChunkSummary,
  ): Promise<boolean> {
    const raster = await client.fieldRaster({
      chartId: chunk.chartId,
      chunkX: chunk.chunkX,
      chunkY: chunk.chunkY,
      chunkZ: chunk.chunkZ,
      field,
      detailLevel: 0,
    });
    // An answer of "nothing here" is a finding; the map draws unsurveyed ground
    // rather than holding a stale lattice over it.
    const key = rasterKey(field, chunk);
    if (raster === undefined) return this.rasters.delete(key);
    while (this.rasters.size >= RASTER_CAPACITY && !this.rasters.has(key)) {
      const oldest = this.rasters.keys().next().value;
      if (oldest === undefined) break;
      this.rasters.delete(oldest);
    }
    this.rasters.set(key, raster);
    return true;
  }

  private scheduleTick(delay: number): void {
    this.timer = window.setTimeout(() => {
      void (async () => {
        if (!this.store.snapshot.running) return;
        const advanced = await this.advance(this.store.snapshot.batch);
        if (advanced && this.store.snapshot.running) this.scheduleTick(RUN_INTERVAL_MS);
      })();
    }, delay);
  }

  private async advance(ticks: number): Promise<boolean> {
    const client = this.client;
    if (client === undefined || this.store.snapshot.connection !== "connected") return false;
    try {
      const update = await client.advance(ticks);
      const current = this.store.snapshot;
      this.store.patch({
        summary: update.summary,
        previous: current.summary,
        history: appendBounded(current.history, update.summary, HISTORY_CAPACITY),
        error: undefined,
        errorKind: undefined,
      });
      if (this.demand.demanded("world")) await this.refreshWorld();
      return true;
    } catch (cause) {
      this.stop();
      this.store.patch({ error: message(cause), errorKind: errorKindOf(cause) });
      return false;
    }
  }

  private forgetRasters(): void {
    if (this.rasters.size === 0) return;
    this.rasters.clear();
    this.store.patch({ rasters: new Map() });
  }

  /**
   * A world frame fetches one raster per chunk per field, which would otherwise
   * bury every other exchange in the instrument log. Consecutive raster requests
   * are folded into one entry that states how many there were and what they cost
   * — a reduction of the log, not of what was sent.
   */
  private recordExchange(exchange: Exchange): void {
    this.store.patch((current) => {
      const previous = current.exchanges[current.exchanges.length - 1];
      if (
        exchange.command === "observer_field_raster" &&
        previous?.command === "observer_field_raster" &&
        previous.outcome === exchange.outcome
      ) {
        // These requests are issued together (Promise.all) but serialise on the
        // session's Tauri mutex, so each raw durationMs already carries the queue
        // wait of every call ahead of it. Summing them counts that wait n times
        // over. The batch's real cost is its wall-clock span, first call's start
        // to this call's finish; `previous.at - previous.durationMs` recovers
        // that start and is preserved unchanged through every fold in the run.
        const batchStart = previous.at - previous.durationMs;
        const folded: Exchange = {
          ...previous,
          count: (previous.count ?? 1) + 1,
          requestBytes: previous.requestBytes + exchange.requestBytes,
          responseBytes: previous.responseBytes + exchange.responseBytes,
          durationMs: exchange.at - batchStart,
          detail: exchange.detail,
          at: exchange.at,
        };
        return { exchanges: [...current.exchanges.slice(0, -1), folded] };
      }
      return { exchanges: appendBounded(current.exchanges, exchange, EXCHANGE_CAPACITY) };
    });
  }
}

function appendBounded<T>(items: readonly T[], item: T, capacity: number): T[] {
  const next = items.length >= capacity ? items.slice(items.length - capacity + 1) : items.slice();
  next.push(item);
  return next;
}

function message(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

/**
 * Whether a failure is the runtime's export cap rather than a fault.
 *
 * No accepted state may become unexportable, so a tick whose result would not
 * fit the envelope is refused whole and the session is left exactly as it was.
 * That is a limit being enforced, and an instrument that reported it in the same
 * red as a broken bridge would be miscategorising the one interesting thing
 * about it.
 *
 * Recognised by the wording the runtime uses, because the failure crosses the
 * bridge as text. A rewording would fall back to the generic notice, which is
 * the safe direction to be wrong in.
 */
function errorKindOf(cause: unknown): SessionState["errorKind"] {
  const text = message(cause);
  return text.includes("snapshot envelope would encode") ||
    text.includes("hydrology snapshot section would encode")
    ? "export-cap"
    : undefined;
}

/**
 * Channel resolution.
 *
 * Production has exactly one channel: the Tauri bridge. When it is absent the observer
 * reports itself unavailable and shows no data — it never substitutes fabricated state
 * (INV-039). In a development build only, a recorded capture of real runtime output may be
 * replayed instead; see `src/dev/replayChannel.ts`. That path is compiled out of production
 * bundles and every surface it feeds is marked as a replay.
 */
export function resolveChannel(devChannel?: ByteChannel): ByteChannel | undefined {
  if (hasTauriTransport()) return tauriChannel;
  if (import.meta.env.DEV && devChannel !== undefined) return devChannel;
  return undefined;
}
