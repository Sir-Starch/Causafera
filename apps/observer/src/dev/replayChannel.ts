/**
 * Development-only replay channel.
 *
 * This is **not** demo or fixture data. It replays protocol bytes that a real
 * `ObserverSession` produced, captured by
 * `cargo run -p causafera-observer --example capture_replay`. The bytes are decoded by the
 * same production codec, so what the interface renders is authentic runtime output.
 *
 * It exists because frontend work often happens without a graphical desktop session. It is
 * unavailable in production builds, requires the capture file to be present, and every
 * surface it feeds is marked as a replay in the interface chrome.
 *
 * The capture is not committed; run the example to produce it. Without it this module
 * returns `undefined` and the observer reports itself unattached, as it must (INV-039).
 */

import type { ByteChannel, ObserverCommand } from "../observer/transport";

const CAPTURE_URL = "/__causafera_dev__/replay.json";

interface CaptureFrame {
  ticks: number;
  runtime: number[];
  world: number[];
  /** Keyed `field|chartId:x:y:z`, exactly as the session keys its raster cache. */
  rasters?: Record<string, number[]>;
}

interface Capture {
  seed: number;
  ticksPerFrame: number;
  connect: number[];
  frames: CaptureFrame[];
  explanation: number[];
}

export async function createReplayChannel(): Promise<ByteChannel | undefined> {
  if (!import.meta.env.DEV) return undefined;
  let capture: Capture;
  try {
    const response = await fetch(CAPTURE_URL);
    if (!response.ok) return undefined;
    capture = (await response.json()) as Capture;
  } catch {
    return undefined;
  }
  if (!Array.isArray(capture.frames) || capture.frames.length === 0) return undefined;

  let cursor = 0;
  const bytes = (values: number[]) => Uint8Array.from(values);
  const frame = () => capture.frames[Math.min(cursor, capture.frames.length - 1)]!;

  return {
    label: `replay:${capture.seed}`,
    async invoke(command: ObserverCommand, args?: Record<string, unknown>): Promise<Uint8Array> {
      // A small delay keeps the run loop's pacing realistic while replaying.
      await new Promise((resolve) => setTimeout(resolve, 8));
      switch (command) {
        case "observer_connect":
          return bytes(capture.connect);
        case "observer_open_stream":
          cursor = 0;
          return bytes(frame().runtime);
        case "observer_reset":
          cursor = 0;
          return bytes(frame().runtime);
        case "observer_advance":
          cursor = Math.min(cursor + 1, capture.frames.length - 1);
          return bytes(frame().runtime);
        case "observer_query":
          return bytes(frame().world);
        case "observer_field_raster": {
          const request = args?.request;
          if (!(request instanceof Uint8Array)) {
            throw new Error("raster replay needs the encoded request");
          }
          const key = rasterKeyOf(request);
          // Terrain is captured once because it does not change per tick, so a
          // key the current frame lacks falls back to the frame that has it.
          const captured = frame().rasters?.[key] ?? capture.frames[0]?.rasters?.[key];
          // A capture taken before rasters existed simply has none; the map
          // then draws what it has rather than being fed something invented.
          return captured === undefined ? notAvailable(request) : bytes(captured);
        }
        case "observer_analyze":
          return bytes(capture.explanation);
        default:
          throw new Error(`replay channel does not implement ${command}`);
      }
    },
  };
}

/**
 * The capture is keyed by field and chunk, so the replay has to read those back
 * out of the request. This is a deliberately small protobuf scan rather than a
 * second decoder: it reads the query envelope's payload and the six scalars
 * inside it, and knows nothing else about the wire.
 */
function rasterKeyOf(request: Uint8Array): string {
  const query = scanFields(request);
  const payload = query.bytes.get(5);
  if (payload === undefined) return "";
  const fields = scanFields(payload).varints;
  const zigzag = (value: bigint | undefined): number =>
    value === undefined ? 0 : Number((value >> 1n) ^ -(value & 1n));
  return (
    `${Number(fields.get(5) ?? 0n)}|${fields.get(1) ?? 0n}:` +
    `${zigzag(fields.get(2))}:${zigzag(fields.get(3))}:${zigzag(fields.get(4))}`
  );
}

/** The request id, echoed back with `NotAvailable` exactly as the session would. */
function notAvailable(request: Uint8Array): Uint8Array {
  const requestId = scanFields(request).varints.get(1) ?? 0n;
  const output: number[] = [];
  const varint = (value: bigint) => {
    let remaining = value;
    while (remaining >= 0x80n) {
      output.push(Number(remaining & 0x7fn) | 0x80);
      remaining >>= 7n;
    }
    output.push(Number(remaining));
  };
  varint(1n << 3n);
  varint(requestId);
  varint(2n << 3n);
  varint(1n);
  varint(3n << 3n);
  varint(4n); // QueryStatus::NotAvailable
  return Uint8Array.from(output);
}

function scanFields(input: Uint8Array): {
  varints: Map<number, bigint>;
  bytes: Map<number, Uint8Array>;
} {
  const varints = new Map<number, bigint>();
  const byteFields = new Map<number, Uint8Array>();
  let offset = 0;
  const readVarint = (): bigint => {
    let value = 0n;
    for (let shift = 0n; shift <= 63n; shift += 7n) {
      const byte = input[offset++];
      if (byte === undefined) throw new Error("truncated replay request");
      value |= BigInt(byte & 0x7f) << shift;
      if ((byte & 0x80) === 0) return value;
    }
    throw new Error("invalid replay request varint");
  };
  while (offset < input.length) {
    const key = readVarint();
    const field = Number(key >> 3n);
    const wire = Number(key & 7n);
    if (wire === 0) varints.set(field, readVarint());
    else if (wire === 2) {
      const length = Number(readVarint());
      byteFields.set(field, input.slice(offset, offset + length));
      offset += length;
    } else if (wire === 1) offset += 8;
    else if (wire === 5) offset += 4;
    else throw new Error(`unsupported replay wire type ${wire}`);
  }
  return { varints, bytes: byteFields };
}
