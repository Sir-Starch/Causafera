/**
 * Presentation formatting.
 *
 * Formatting is presentation only. It never participates in simulation state and never
 * changes across locales in a way that could imply a different measured value (INV-006).
 */

/**
 * The locales the observer presents itself in.
 *
 * These tags are presentation identity, not simulation identity. They travel to the protocol
 * handler on connect so the runtime knows which locale is observing, and nothing downstream of
 * that may branch on them in a way that reaches authoritative state (INV-006, INV-007).
 *
 * `zh-Hans` carries a script subtag rather than a region because the distinction that matters
 * for Chinese is the script, not the country.
 */
export type ObserverLocale = "en-US" | "ru-RU" | "zh-Hans" | "de-DE" | "es-ES";

const integerFormats = new Map<string, Intl.NumberFormat>();

function integerFormat(locale: ObserverLocale): Intl.NumberFormat {
  let format = integerFormats.get(locale);
  if (format === undefined) {
    format = new Intl.NumberFormat(locale, { useGrouping: true, maximumFractionDigits: 0 });
    integerFormats.set(locale, format);
  }
  return format;
}

/** Group an integer for reading. Accepts bigint so 64-bit counters never lose precision. */
export function formatInteger(value: bigint | number, locale: ObserverLocale): string {
  return integerFormat(locale).format(typeof value === "bigint" ? value : Math.round(value));
}

/** Compact magnitude for dense readouts: 20 270 → 20.3k. Never used where exactness matters. */
export function formatCompact(value: bigint | number, locale: ObserverLocale): string {
  const numeric = typeof value === "bigint" ? Number(value) : value;
  const magnitude = Math.abs(numeric);
  if (magnitude < 10_000) return formatInteger(Math.round(numeric), locale);
  if (magnitude < 1_000_000) return `${(numeric / 1000).toFixed(1)}k`;
  if (magnitude < 1_000_000_000) return `${(numeric / 1_000_000).toFixed(2)}M`;
  return `${(numeric / 1_000_000_000).toFixed(2)}G`;
}

/** Millimetre elevations are stored as integers; metres are the readable unit. */
export function formatMillimetresAsMetres(value: number, fractionDigits = 2): string {
  return (value / 1000).toFixed(fractionDigits);
}

export function formatPercent(fraction: number, fractionDigits = 0): string {
  return `${(fraction * 100).toFixed(fractionDigits)} %`;
}

export function formatDuration(milliseconds: number): string {
  if (milliseconds < 1) return "<1 ms";
  if (milliseconds < 1000) return `${milliseconds.toFixed(milliseconds < 10 ? 1 : 0)} ms`;
  return `${(milliseconds / 1000).toFixed(2)} s`;
}

export function formatBytes(value: number, locale: ObserverLocale): string {
  if (value < 1024) return `${formatInteger(value, locale)} B`;
  return `${(value / 1024).toFixed(1)} KiB`;
}

/** Chart-qualified chunk address. Never rendered as a seamless global coordinate (INV-036). */
export function formatChunkAddress(chunk: {
  chartId: bigint;
  chunkX: number;
  chunkY: number;
  chunkZ: number;
}): string {
  return `C${chunk.chartId} · ${chunk.chunkX}, ${chunk.chunkY}, ${chunk.chunkZ}`;
}

/** Split a digest into byte pairs so divergence is visible without reading the whole hash. */
export function digestPairs(value: Uint8Array, count: number): string[] {
  const pairs: string[] = [];
  for (let index = 0; index < Math.min(count, value.length); index += 1) {
    pairs.push(value[index]!.toString(16).padStart(2, "0"));
  }
  return pairs;
}

export function formatTraceId(value: bigint): string {
  return `#${value}`;
}
