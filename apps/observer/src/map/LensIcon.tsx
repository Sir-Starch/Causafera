/**
 * Lens marks.
 *
 * One drawn glyph per lens, in the same engraved line-work as the rest of the instrument. A
 * glyph says what class of information the lens carries; the name arrives on hover. An
 * unrecognised lens falls back to a survey mark rather than to nothing, so a lens added later
 * is usable before anyone draws its icon.
 */

const PATHS: Record<string, string> = {
  // Geography — a relief profile, a measured range, a rough edge, nested isolines, a frond.
  relief: "M2 15 L7 6 L11 11 L14 7 L18 15",
  "relief-range": "M10 3v14M7 6l3-3 3 3M7 14l3 3 3-3M2 3h4M2 17h4M14 3h4M14 17h4",
  roughness: "M2 12l2-3 2 4 2-5 2 6 2-4 2 3 2-5 2 4",
  contours: "M10 10m-2.5 0a2.5 2.5 0 1 0 5 0a2.5 2.5 0 1 0-5 0M10 10m-5.5 0a5.5 2.8 0 1 0 11 0a5.5 2.8 0 1 0-11 0M10 10m-8 0a8 4.6 0 1 0 16 0a8 4.6 0 1 0-16 0",
  ecology: "M10 17V7M10 7c0-3 2.5-4.5 5.5-4.5C15.5 6 13 8 10 8M10 10c0-2.5-2-4-4.5-4C5.5 9 7.5 11 10 11",
  // Material — a lattice with one marked cell.
  surface: "M3 3h14v14H3zM3 8h14M3 13h14M8 3v14M13 3v14M9 9.5l1.5-1.5 1.5 1.5-1.5 1.5z",
  // Mana — a radiant source, a directed difference, a gate.
  mana: "M10 10m-2 0a2 2 0 1 0 4 0a2 2 0 1 0-4 0M10 2v3M10 15v3M2 10h3M15 10h3M4.5 4.5l2 2M13.5 13.5l2 2M15.5 4.5l-2 2M6.5 13.5l-2 2",
  "mana-gradient": "M3 10h13M12 6l4 4-4 4M3 5v10",
  gates: "M4 3v14M16 3v14M4 7h5M11 7h5M4 13h4M12 13h4",
  // Life — a settlement cluster.
  population: "M10 10m-6 0a6 6 0 1 0 12 0a6 6 0 1 0-12 0M10 10m-1.6 0a1.6 1.6 0 1 0 3.2 0a1.6 1.6 0 1 0-3.2 0M14.5 5.5m-1 0a1 1 0 1 0 2 0a1 1 0 1 0-2 0M5.5 14m-1 0a1 1 0 1 0 2 0a1 1 0 1 0-2 0",
  // Causality — committed events, an anchor, resolution steps, an ancestry fork.
  "causal-activity": "M4 16L8 9l3 4 5-9M4 16h12",
  "trace-anchors": "M10 4v12M6 8h8M10 16c-3 0-5-2-5-4M10 16c3 0 5-2 5-4M10 4m-1.5 0a1.5 1.5 0 1 0 3 0a1.5 1.5 0 1 0-3 0",
  resolution: "M3 3h14v14H3zM6 6h8v8H6zM8.5 8.5h3v3h-3z",
  provenance: "M10 17v-4M10 13L5 8V3M10 13l5-5V3M5 3m-1.5 0a1.5 1.5 0 1 0 3 0a1.5 1.5 0 1 0-3 0M15 3m-1.5 0a1.5 1.5 0 1 0 3 0a1.5 1.5 0 1 0-3 0M10 17m-1.5 0a1.5 1.5 0 1 0 3 0a1.5 1.5 0 1 0-3 0",
  // Cognition — an actor with a heading, a partly closed arc, an utterance.
  agents: "M10 6m-2.5 0a2.5 2.5 0 1 0 5 0a2.5 2.5 0 1 0-5 0M5 17c0-3 2.2-5 5-5s5 2 5 5",
  knowledge: "M10 10m-7 0a7 7 0 1 1 7 7M10 10m-3.2 0a3.2 3.2 0 1 0 6.4 0a3.2 3.2 0 1 0-6.4 0",
  language: "M3 5h14v8H8l-3 3v-3H3zM6 9h8",
  // Society — a network, a transmitted pattern, an exchange.
  social:
    "M5 5m-2 0a2 2 0 1 0 4 0a2 2 0 1 0-4 0M15 5m-2 0a2 2 0 1 0 4 0a2 2 0 1 0-4 0M10 15m-2 0a2 2 0 1 0 4 0a2 2 0 1 0-4 0M6.5 6.5L9 13M13.5 6.5L11 13M7 5h6",
  practices: "M4 7h5l2 6h5M4 7a2.5 2.5 0 1 1 0-.1M16 13a2.5 2.5 0 1 1 0 .1M7 16h6",
  economy: "M3 7h11l-3-3M17 13H6l3 3",
};

const FALLBACK = "M10 3v14M3 10h14M10 10m-5 0a5 5 0 1 0 10 0a5 5 0 1 0-10 0";

export function LensIcon({ lensId, size = 17 }: { lensId: string; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d={PATHS[lensId] ?? FALLBACK} />
    </svg>
  );
}
