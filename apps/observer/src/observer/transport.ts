/**
 * Observer transport.
 *
 * Every byte the frontend sees crosses this boundary. The codec in
 * `@causafera/observer-protocol` decodes it; nothing else in the application is allowed to
 * construct observer payloads. The client records each exchange so the instrument log can
 * show real wire behaviour rather than a simulated one.
 */

import {
  OBSERVER_PROTOCOL_V1,
  QueryKind,
  QueryStatus,
  StreamKind,
  decodeConnectResponse,
  decodeExplanationReport,
  decodeFieldRaster,
  decodeQueryResponse,
  decodeRuntimeSummary,
  decodeStreamEnvelope,
  decodeWorldChunkSnapshot,
  digestHex,
  encodeConnectRequest,
  encodeFieldRasterQuery,
  encodeQuery,
  type ConnectResponse,
  type ExplanationReport,
  type FieldRaster,
  type FieldRasterRequest,
  type RuntimeSummary,
  type WorldChunkSnapshot,
} from "@causafera/observer-protocol";

declare global {
  interface Window {
    __TAURI__?: {
      core: {
        invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
      };
    };
  }
}

export type ObserverCommand =
  | "observer_connect"
  | "observer_open_stream"
  | "observer_advance"
  | "observer_query"
  | "observer_field_raster"
  | "observer_analyze"
  | "observer_reset";

/** A raw byte channel. The Tauri bridge is the only production implementation. */
export interface ByteChannel {
  readonly label: string;
  invoke(command: ObserverCommand, args?: Record<string, unknown>): Promise<Uint8Array>;
}

export interface RuntimeUpdate {
  summary: RuntimeSummary;
  sequenceNumber: bigint;
  isSnapshot: boolean;
}

export interface Exchange {
  id: number;
  command: ObserverCommand;
  detail?: string;
  requestBytes: number;
  responseBytes: number;
  durationMs: number;
  outcome: "ok" | "failed";
  message?: string;
  at: number;
  /** Exchanges folded into this entry by the session log; absent means one. */
  count?: number;
}

/** Exchange-log labels only; the wire carries the numeric field selector. */
const FieldRasterKindLabel: Record<number, string> = {
  1: "elevation",
  2: "roughness",
  3: "mana",
};

export function hasTauriTransport(): boolean {
  return typeof window !== "undefined" && window.__TAURI__?.core.invoke !== undefined;
}

export const tauriChannel: ByteChannel = {
  label: "tauri",
  async invoke(command, args) {
    const invoke = window.__TAURI__?.core.invoke;
    if (invoke === undefined) throw new Error("Tauri observer bridge is unavailable");
    const serialized =
      args === undefined
        ? undefined
        : Object.fromEntries(
            Object.entries(args).map(([key, value]) => [
              key,
              value instanceof Uint8Array ? Array.from(value) : value,
            ]),
          );
    const response = await invoke<number[] | Uint8Array | ArrayBuffer>(command, serialized);
    if (response instanceof ArrayBuffer) return new Uint8Array(response);
    return response instanceof Uint8Array ? response : Uint8Array.from(response as number[]);
  },
};

export class ObserverClient {
  private requestId = 1n;
  private exchangeId = 0;

  constructor(
    private readonly channel: ByteChannel,
    private readonly onExchange: (exchange: Exchange) => void = () => {},
  ) {}

  get transportLabel(): string {
    return this.channel.label;
  }

  async connect(locale: string): Promise<ConnectResponse> {
    const request = encodeConnectRequest({
      supportedProtocolVersions: [OBSERVER_PROTOCOL_V1],
      observerLocale: locale,
    });
    const bytes = await this.exchange("observer_connect", { request }, request.length, locale);
    const response = decodeConnectResponse(bytes);
    if (response.selectedProtocolVersion !== OBSERVER_PROTOCOL_V1) {
      throw new Error(
        `observer negotiated unsupported protocol version ${response.selectedProtocolVersion}`,
      );
    }
    return response;
  }

  async openRuntimeStream(): Promise<RuntimeUpdate> {
    return decodeRuntimeUpdate(await this.exchange("observer_open_stream", undefined, 0));
  }

  async advance(ticks: number): Promise<RuntimeUpdate> {
    return decodeRuntimeUpdate(
      await this.exchange("observer_advance", { ticks }, 0, `${ticks} ticks`),
    );
  }

  async reset(seed: number): Promise<RuntimeUpdate> {
    return decodeRuntimeUpdate(await this.exchange("observer_reset", { seed }, 0, `seed ${seed}`));
  }

  async world(): Promise<WorldChunkSnapshot> {
    return decodeWorldChunkSnapshot(await this.query("observer_query", QueryKind.WorldChunks));
  }

  async analyze(): Promise<ExplanationReport> {
    return decodeExplanationReport(await this.query("observer_analyze", QueryKind.ExplanationIr));
  }

  /**
   * One chunk of one measured lattice.
   *
   * `undefined` means the observer answered that it has nothing for this chunk,
   * which is a finding rather than a failure: ground outside the active set is
   * unsurveyed, and the map draws it as such instead of retrying.
   */
  async fieldRaster(request: FieldRasterRequest): Promise<FieldRaster | undefined> {
    const encoded = encodeFieldRasterQuery(this.nextRequestId(), request);
    const bytes = await this.exchange(
      "observer_field_raster",
      { request: encoded },
      encoded.length,
      `${FieldRasterKindLabel[request.field]} ${request.chunkX},${request.chunkY}`,
    );
    const response = decodeQueryResponse(bytes);
    if (response.protocolVersion !== OBSERVER_PROTOCOL_V1) {
      throw new Error(`unsupported observer response version ${response.protocolVersion}`);
    }
    if (response.status === QueryStatus.NotAvailable) return undefined;
    if (response.status !== QueryStatus.Ok) {
      throw new Error(`observer raster query failed with status ${QueryStatus[response.status]}`);
    }
    return decodeFieldRaster(response.payload);
  }

  private async query(command: ObserverCommand, kind: QueryKind): Promise<Uint8Array> {
    const request = encodeQuery(kind, this.nextRequestId());
    const bytes = await this.exchange(command, { request }, request.length, QueryKind[kind]);
    const response = decodeQueryResponse(bytes);
    if (response.protocolVersion !== OBSERVER_PROTOCOL_V1) {
      throw new Error(`unsupported observer response version ${response.protocolVersion}`);
    }
    if (response.status === QueryStatus.NotAvailable) {
      throw new Error("observer data is not available for this session");
    }
    if (response.status !== QueryStatus.Ok) {
      throw new Error(`observer query failed with status ${QueryStatus[response.status]}`);
    }
    return response.payload;
  }

  private async exchange(
    command: ObserverCommand,
    args: Record<string, unknown> | undefined,
    requestBytes: number,
    detail?: string,
  ): Promise<Uint8Array> {
    const id = (this.exchangeId += 1);
    const started = performance.now();
    try {
      const response = await this.channel.invoke(command, args);
      this.onExchange({
        id,
        command,
        detail,
        requestBytes,
        responseBytes: response.length,
        durationMs: performance.now() - started,
        outcome: "ok",
        at: Date.now(),
      });
      return response;
    } catch (cause) {
      this.onExchange({
        id,
        command,
        detail,
        requestBytes,
        responseBytes: 0,
        durationMs: performance.now() - started,
        outcome: "failed",
        message: cause instanceof Error ? cause.message : String(cause),
        at: Date.now(),
      });
      throw cause;
    }
  }

  private nextRequestId(): bigint {
    const current = this.requestId;
    this.requestId += 1n;
    return current;
  }
}

/**
 * Digest anchors are verified on every runtime update. This is a consistency requirement of
 * the stream contract, not optional validation: a payload that does not match its envelope
 * header is rejected rather than displayed.
 */
function decodeRuntimeUpdate(bytes: Uint8Array): RuntimeUpdate {
  const envelope = decodeStreamEnvelope(bytes);
  if (envelope.kind !== StreamKind.RuntimeSummary) {
    throw new Error(`unexpected stream kind ${envelope.kind}`);
  }
  const summary = decodeRuntimeSummary(envelope.payload);
  if (
    digestHex(summary.physicalDigest) !== digestHex(envelope.header.physicalDigest) ||
    digestHex(summary.historyDigest) !== digestHex(envelope.header.historyDigest)
  ) {
    throw new Error("stream digest anchor does not match runtime payload");
  }
  if (summary.simulationTicks !== envelope.header.simulationTime) {
    throw new Error("stream time does not match runtime payload");
  }
  return {
    summary,
    sequenceNumber: envelope.header.sequenceNumber,
    isSnapshot: envelope.header.isSnapshot,
  };
}
