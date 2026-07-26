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

import type { ExplanationReport, WorldChunkSnapshot } from "@causafera/observer-protocol";
import type { RuntimeSummary } from "@causafera/observer-protocol";

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
  explanation?: ExplanationReport;
  explanationTicks?: bigint;
  history: RuntimeSummary[];
  exchanges: Exchange[];
  error?: string;
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
  history: [],
  exchanges: [],
  running: false,
  analyzing: false,
  batch: 4,
  seed: 0,
  locale: "ru-RU",
};

export class ObserverSessionController {
  readonly store = new Store<SessionState>(initialState);
  readonly demand = new DemandRegistry<Feed>();

  private client?: ObserverClient;
  private started = false;
  private timer?: number;
  private generation = 0;

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
    this.store.patch({ connection: "connecting", error: undefined });
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
      this.store.patch({ connection: "error", error: message(cause), running: false });
    }
  }

  setLocale(locale: ObserverLocale): void {
    if (this.store.snapshot.locale === locale) return;
    this.store.patch({ locale });
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
    this.store.patch({ running: true, error: undefined });
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
      });
      if (this.demand.demanded("world")) await this.refreshWorld();
    } catch (cause) {
      this.store.patch({ error: message(cause) });
    }
  }

  async analyze(): Promise<void> {
    const client = this.client;
    if (client === undefined || this.store.snapshot.connection !== "connected") return;
    if (this.store.snapshot.analyzing) return;
    this.stop();
    this.store.patch({ analyzing: true, error: undefined });
    try {
      const report = await client.analyze();
      this.store.patch({
        explanation: report,
        explanationTicks: this.store.snapshot.summary?.simulationTicks,
      });
    } catch (cause) {
      this.store.patch({ error: message(cause) });
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
    } catch (cause) {
      this.store.patch({ error: message(cause) });
    }
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
      });
      if (this.demand.demanded("world")) await this.refreshWorld();
      return true;
    } catch (cause) {
      this.stop();
      this.store.patch({ error: message(cause) });
      return false;
    }
  }

  private recordExchange(exchange: Exchange): void {
    this.store.patch((current) => ({
      exchanges: appendBounded(current.exchanges, exchange, EXCHANGE_CAPACITY),
    }));
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
