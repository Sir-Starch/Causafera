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
    async invoke(command: ObserverCommand): Promise<Uint8Array> {
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
        case "observer_analyze":
          return bytes(capture.explanation);
        default:
          throw new Error(`replay channel does not implement ${command}`);
      }
    },
  };
}
