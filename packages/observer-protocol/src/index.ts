/** Observer v1 client codec. Field numbers are defined by proto/ontopolis/observer/v1. */
export const OBSERVER_PROTOCOL_V1 = 1;

export interface ConnectRequest {
  supportedProtocolVersions: number[];
  observerLocale: string;
}

export interface ConnectResponse {
  selectedProtocolVersion: number;
  currentTime: { ticks: bigint };
  capabilities: number[];
}

export enum QueryKind {
  RuntimeSummary = 1,
  ExplanationIr = 2,
}

export enum QueryStatus {
  Ok = 1,
  InvalidRequest = 2,
  Unsupported = 3,
  NotAvailable = 4,
}

export interface RuntimeSummary {
  simulationTicks: bigint;
  digestSchemaVersion: number;
  physicalDigest: Uint8Array;
  historyDigest: Uint8Array;
  manaTotal: bigint;
  manaMaximum: bigint;
  activeChunkCount: number;
  resolutionRelevance: bigint;
  resolutionLevel: number;
  causalTraceCount: bigint;
  actorCount: number;
  populationTotal: bigint;
  latestTraceId: bigint;
}

export function encodeRuntimeSummaryQuery(requestId: bigint): Uint8Array {
  const output: number[] = [];
  fieldVarint(output, 1, requestId);
  fieldVarint(output, 2, BigInt(OBSERVER_PROTOCOL_V1));
  fieldVarint(output, 3, BigInt(QueryKind.RuntimeSummary));
  return Uint8Array.from(output);
}

export function decodeRuntimeSummary(input: Uint8Array): RuntimeSummary {
  const cursor = new Cursor(input);
  const values = new Map<number, bigint>();
  let physicalDigest = new Uint8Array();
  let historyDigest = new Uint8Array();
  while (!cursor.empty) {
    const [field, wire] = cursor.key();
    if (wire === 0) values.set(field, cursor.varint());
    else if (wire === 2 && field === 3) physicalDigest = cursor.bytes();
    else if (wire === 2 && field === 4) historyDigest = cursor.bytes();
    else cursor.skip(wire);
  }
  if (physicalDigest.length !== 32 || historyDigest.length !== 32)
    throw new Error("observer digest must contain 32 bytes");
  const value = (field: number): bigint => {
    const found = values.get(field);
    if (found === undefined) throw new Error(`missing RuntimeSummary field ${field}`);
    return found;
  };
  return {
    simulationTicks: value(1),
    digestSchemaVersion: Number(value(2)),
    physicalDigest,
    historyDigest,
    manaTotal: BigInt.asIntN(64, value(5)),
    manaMaximum: BigInt.asIntN(64, value(6)),
    activeChunkCount: Number(value(7)),
    resolutionRelevance: BigInt.asIntN(64, value(8)),
    resolutionLevel: Number(value(9)),
    causalTraceCount: value(10),
    actorCount: Number(value(11)),
    populationTotal: value(12),
    latestTraceId: value(23),
  };
}

function fieldVarint(output: number[], field: number, value: bigint): void {
  writeVarint(output, BigInt(field << 3));
  writeVarint(output, value);
}

function writeVarint(output: number[], initial: bigint): void {
  let value = initial;
  while (value >= 0x80n) {
    output.push(Number(value & 0x7fn) | 0x80);
    value >>= 7n;
  }
  output.push(Number(value));
}

class Cursor {
  private offset = 0;
  constructor(private readonly input: Uint8Array) {}
  get empty(): boolean { return this.offset === this.input.length; }
  varint(): bigint {
    let value = 0n;
    for (let shift = 0n; shift <= 63n; shift += 7n) {
      const byte = this.input[this.offset++];
      if (byte === undefined) throw new Error("truncated protobuf varint");
      value |= BigInt(byte & 0x7f) << shift;
      if ((byte & 0x80) === 0) return value;
    }
    throw new Error("invalid protobuf varint");
  }
  key(): [number, number] {
    const key = this.varint();
    return [Number(key >> 3n), Number(key & 7n)];
  }
  bytes(): Uint8Array {
    const length = Number(this.varint());
    const end = this.offset + length;
    if (end > this.input.length) throw new Error("truncated protobuf bytes");
    const bytes = this.input.slice(this.offset, end);
    this.offset = end;
    return bytes;
  }
  skip(wire: number): void {
    if (wire === 0) { this.varint(); return; }
    if (wire === 1) { this.offset += 8; return; }
    if (wire === 2) { this.bytes(); return; }
    if (wire === 5) { this.offset += 4; return; }
    throw new Error(`unsupported protobuf wire type ${wire}`);
  }
}
