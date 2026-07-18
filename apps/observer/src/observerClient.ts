import {
  OBSERVER_PROTOCOL_V1,
  QueryKind,
  QueryStatus,
  StreamKind,
  decodeConnectResponse,
  decodeExplanationReport,
  decodeQueryResponse,
  decodeRuntimeSummary,
  decodeStreamEnvelope,
  decodeWorldChunkSnapshot,
  digestHex,
  encodeConnectRequest,
  encodeQuery,
  type ConnectResponse,
  type ExplanationReport,
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

export interface RuntimeUpdate {
  summary: RuntimeSummary;
  sequenceNumber: bigint;
  isSnapshot: boolean;
}

export function hasTauriTransport(): boolean {
  return typeof window !== "undefined" && window.__TAURI__?.core.invoke !== undefined;
}

export class ObserverClient {
  private requestId = 1n;

  async connect(locale: string): Promise<ConnectResponse> {
    const bytes = encodeConnectRequest({
      supportedProtocolVersions: [OBSERVER_PROTOCOL_V1],
      observerLocale: locale,
    });
    return decodeConnectResponse(await this.invokeBytes("observer_connect", { request: bytes }));
  }

  async openRuntimeStream(): Promise<RuntimeUpdate> {
    return this.decodeRuntimeUpdate(await this.invokeBytes("observer_open_stream"));
  }

  async advance(ticks: number): Promise<RuntimeUpdate> {
    return this.decodeRuntimeUpdate(await this.invokeBytes("observer_advance", { ticks }));
  }

  async reset(seed: number): Promise<RuntimeUpdate> {
    return this.decodeRuntimeUpdate(await this.invokeBytes("observer_reset", { seed }));
  }

  async world(): Promise<WorldChunkSnapshot> {
    const response = await this.query("observer_query", QueryKind.WorldChunks);
    return decodeWorldChunkSnapshot(response);
  }

  async analyze(): Promise<ExplanationReport> {
    const response = await this.query("observer_analyze", QueryKind.ExplanationIr);
    return decodeExplanationReport(response);
  }

  private async query(command: string, kind: QueryKind): Promise<Uint8Array> {
    const request = encodeQuery(kind, this.nextRequestId());
    const response = decodeQueryResponse(await this.invokeBytes(command, { request }));
    if (response.protocolVersion !== OBSERVER_PROTOCOL_V1) {
      throw new Error(`unsupported observer response version ${response.protocolVersion}`);
    }
    if (response.status === QueryStatus.NotAvailable) {
      throw new Error("observer data is not available for this session");
    }
    if (response.status !== QueryStatus.Ok) {
      throw new Error(`observer query failed with status ${response.status}`);
    }
    return response.payload;
  }

  private decodeRuntimeUpdate(bytes: Uint8Array): RuntimeUpdate {
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

  private nextRequestId(): bigint {
    const current = this.requestId;
    this.requestId += 1n;
    return current;
  }

  private async invokeBytes(command: string, args?: Record<string, unknown>): Promise<Uint8Array> {
    const invoke = window.__TAURI__?.core.invoke;
    if (invoke === undefined) throw new Error("Tauri observer bridge is unavailable");
    const serializedArgs = args === undefined ? undefined : mapByteArguments(args);
    const response = await invoke<number[] | Uint8Array>(command, serializedArgs);
    return response instanceof Uint8Array ? response : Uint8Array.from(response);
  }
}

function mapByteArguments(args: Record<string, unknown>): Record<string, unknown> {
  return Object.fromEntries(
    Object.entries(args).map(([key, value]) => [
      key,
      value instanceof Uint8Array ? Array.from(value) : value,
    ]),
  );
}
