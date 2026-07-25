/**
 * The chart sheet.
 *
 * A black outline map drawn beneath the whole application: coastlines, engraved water
 * lining, interior contour rings, a graticule, rhumb lines from two compass nodes, soundings,
 * and hatched unsurveyed ground where the survey stops.
 *
 * Every line is chart furniture. Nothing here encodes simulation state, and nothing here is
 * random: the coastlines come from fixed sums of sinusoids, so the sheet is identical in every
 * session and can never be mistaken for data (INV-022, INV-039).
 */

import { useMemo } from "react";

const WIDTH = 1600;
const HEIGHT = 1000;
// Enough samples for the highest harmonic; more only inflates the static DOM.
const SAMPLES = 168;

/** One landmass: a radius function built from fixed harmonics, plus where the survey stops. */
interface Land {
  cx: number;
  cy: number;
  radius: number;
  stretch: number;
  harmonics: [amplitude: number, frequency: number, phase: number][];
  /** Arcs, in turns, that the survey never closed. Drawn dashed instead of solid. */
  unsurveyed: [start: number, end: number][];
  contours: number;
  lining: number;
}

const LANDS: Land[] = [
  {
    cx: 470,
    cy: 430,
    radius: 300,
    stretch: 1.22,
    harmonics: [
      [0.14, 2, 0.6],
      [0.09, 3, 2.1],
      [0.055, 5, 4.4],
      [0.03, 8, 1.2],
      [0.018, 13, 3.7],
    ],
    unsurveyed: [
      [0.16, 0.31],
      [0.62, 0.71],
    ],
    contours: 5,
    lining: 4,
  },
  {
    cx: 1230,
    cy: 690,
    radius: 210,
    stretch: 0.92,
    harmonics: [
      [0.17, 2, 3.4],
      [0.08, 4, 0.9],
      [0.05, 6, 2.6],
      [0.025, 11, 5.1],
    ],
    unsurveyed: [[0.78, 0.97]],
    contours: 4,
    lining: 3,
  },
  {
    cx: 1130,
    cy: 190,
    radius: 96,
    stretch: 1.35,
    harmonics: [
      [0.2, 3, 1.7],
      [0.09, 5, 4.0],
      [0.04, 9, 0.4],
    ],
    unsurveyed: [],
    contours: 2,
    lining: 2,
  },
  {
    cx: 245,
    cy: 855,
    radius: 62,
    stretch: 1.1,
    harmonics: [
      [0.22, 2, 2.3],
      [0.1, 4, 5.5],
    ],
    unsurveyed: [],
    contours: 1,
    lining: 2,
  },
];

function radius(land: Land, turn: number, scale: number): number {
  const theta = turn * Math.PI * 2;
  let value = 1;
  for (const [amplitude, frequency, phase] of land.harmonics) {
    value += amplitude * Math.sin(frequency * theta + phase);
  }
  return land.radius * scale * value;
}

function point(land: Land, turn: number, scale: number, swell = 0): [number, number] {
  const theta = turn * Math.PI * 2;
  const r = radius(land, turn, scale) + swell;
  return [land.cx + r * Math.cos(theta) * land.stretch, land.cy + r * Math.sin(theta)];
}

function pathFrom(points: [number, number][], close: boolean): string {
  if (points.length === 0) return "";
  const parts = points.map(
    ([x, y], index) => `${index === 0 ? "M" : "L"}${x.toFixed(1)} ${y.toFixed(1)}`,
  );
  return close ? `${parts.join(" ")} Z` : parts.join(" ");
}

/** A closed ring at `scale`, offset outwards by `swell`. */
function ring(land: Land, scale: number, swell = 0): string {
  const points: [number, number][] = [];
  for (let index = 0; index <= SAMPLES; index += 1) {
    points.push(point(land, index / SAMPLES, scale, swell));
  }
  return pathFrom(points, true);
}

function inUnsurveyed(land: Land, turn: number): boolean {
  return land.unsurveyed.some(([start, end]) => turn >= start && turn <= end);
}

/** The coastline split into the arcs the survey closed and the arcs it did not. */
function coastArcs(land: Land): { surveyed: string[]; unsurveyed: string[] } {
  const surveyed: string[] = [];
  const unsurveyed: string[] = [];
  let run: [number, number][] = [];
  let runUnsurveyed = inUnsurveyed(land, 0);

  const flush = () => {
    if (run.length > 1) (runUnsurveyed ? unsurveyed : surveyed).push(pathFrom(run, false));
    run = [];
  };

  for (let index = 0; index <= SAMPLES; index += 1) {
    const turn = index / SAMPLES;
    const state = inUnsurveyed(land, turn);
    if (state !== runUnsurveyed) {
      run.push(point(land, turn, 1));
      flush();
      runUnsurveyed = state;
    }
    run.push(point(land, turn, 1));
  }
  flush();
  return { surveyed, unsurveyed };
}

/** Portolan rhumb lines radiating from a compass node. */
function rhumbs(cx: number, cy: number, length: number, count: number): string[] {
  const lines: string[] = [];
  for (let index = 0; index < count; index += 1) {
    const theta = (index / count) * Math.PI * 2;
    lines.push(
      `M${cx.toFixed(1)} ${cy.toFixed(1)} L${(cx + Math.cos(theta) * length).toFixed(1)} ${(
        cy +
        Math.sin(theta) * length
      ).toFixed(1)}`,
    );
  }
  return lines;
}

/** Depth soundings scattered on a fixed lattice, skipping anything that falls on land. */
function soundings(): [number, number][] {
  const marks: [number, number][] = [];
  for (let x = 90; x < WIDTH; x += 118) {
    for (let y = 70; y < HEIGHT; y += 104) {
      const jitterX = ((x * 7 + y * 13) % 37) - 18;
      const jitterY = ((x * 11 + y * 5) % 31) - 15;
      const px = x + jitterX;
      const py = y + jitterY;
      const onLand = LANDS.some((land) => {
        const dx = (px - land.cx) / land.stretch;
        const dy = py - land.cy;
        const turn = (Math.atan2(dy, dx) / (Math.PI * 2) + 1) % 1;
        return Math.hypot(dx, dy) < radius(land, turn, 1) + 26;
      });
      if (!onLand) marks.push([px, py]);
    }
  }
  return marks;
}

export function TerraIncognita() {
  const sheet = useMemo(() => {
    return {
      lands: LANDS.map((land) => ({
        coast: coastArcs(land),
        contours: Array.from({ length: land.contours }, (_, index) =>
          ring(land, 1 - (index + 1) * (0.62 / (land.contours + 1))),
        ),
        lining: Array.from({ length: land.lining }, (_, index) =>
          ring(land, 1, 13 + index * (15 + index * 5)),
        ),
      })),
      soundings: soundings(),
      rhumbLines: [...rhumbs(1230, 690, 1500, 16), ...rhumbs(470, 430, 1200, 16)],
    };
  }, []);

  return (
    <svg
      className="terra-sheet"
      viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
      preserveAspectRatio="xMidYMid slice"
      aria-hidden="true"
    >
      <defs>
        {/* Unsurveyed ground reads as hatching, the same mark the interface uses for
            unknown evidence. */}
        <pattern id="terra-hatch" width="9" height="9" patternUnits="userSpaceOnUse" patternTransform="rotate(35)">
          <line x1="0" y1="0" x2="0" y2="9" stroke="currentColor" strokeWidth="0.6" opacity="0.5" />
        </pattern>
        <radialGradient id="terra-fade" cx="38%" cy="30%" r="78%">
          <stop offset="0%" stopColor="#fff" stopOpacity="1" />
          <stop offset="62%" stopColor="#fff" stopOpacity="0.55" />
          <stop offset="100%" stopColor="#fff" stopOpacity="0.12" />
        </radialGradient>
        <mask id="terra-mask">
          <rect width={WIDTH} height={HEIGHT} fill="url(#terra-fade)" />
        </mask>
      </defs>

      <g mask="url(#terra-mask)" fill="none" stroke="currentColor" strokeLinejoin="round">
        {/* Graticule: meridians and parallels bowed to suggest a projection. */}
        <g className="terra-graticule">
          {Array.from({ length: 11 }, (_, index) => {
            const x = (index / 10) * WIDTH;
            const bow = (x - WIDTH / 2) * 0.06;
            return (
              <path
                key={`m${index}`}
                d={`M${x - bow} 0 Q${x + bow} ${HEIGHT / 2} ${x - bow} ${HEIGHT}`}
              />
            );
          })}
          {Array.from({ length: 7 }, (_, index) => {
            const y = (index / 6) * HEIGHT;
            const bow = (y - HEIGHT / 2) * 0.05;
            return (
              <path
                key={`p${index}`}
                d={`M0 ${y + bow} Q${WIDTH / 2} ${y - bow} ${WIDTH} ${y + bow}`}
              />
            );
          })}
        </g>

        <g className="terra-rhumbs">
          {sheet.rhumbLines.map((line, index) => (
            <path key={index} d={line} />
          ))}
        </g>

        {/* Engraved water lining: offset rings hugging every coast. */}
        <g className="terra-lining">
          {sheet.lands.flatMap((land, landIndex) =>
            land.lining.map((line, index) => (
              <path key={`${landIndex}-${index}`} d={line} opacity={0.62 - index * 0.14} />
            )),
          )}
        </g>

        <g className="terra-contours">
          {sheet.lands.flatMap((land, landIndex) =>
            land.contours.map((line, index) => (
              <path key={`${landIndex}-${index}`} d={line} />
            )),
          )}
        </g>

        <g className="terra-coast">
          {sheet.lands.flatMap((land, landIndex) =>
            land.coast.surveyed.map((line, index) => (
              <path key={`${landIndex}-${index}`} d={line} />
            )),
          )}
        </g>

        {/* Where the coast was never closed, the line breaks into survey dashes. */}
        <g className="terra-coast terra-coast--open">
          {sheet.lands.flatMap((land, landIndex) =>
            land.coast.unsurveyed.map((line, index) => (
              <path key={`${landIndex}-${index}`} d={line} />
            )),
          )}
        </g>

        <g className="terra-hatch-field">
          <path d={`M${WIDTH * 0.62} 0 L${WIDTH} 0 L${WIDTH} ${HEIGHT * 0.3} Z`} fill="url(#terra-hatch)" stroke="none" />
          <path d={`M0 ${HEIGHT} L0 ${HEIGHT * 0.72} L${WIDTH * 0.14} ${HEIGHT} Z`} fill="url(#terra-hatch)" stroke="none" />
        </g>

        <g className="terra-soundings">
          {sheet.soundings.map(([x, y], index) =>
            index % 5 === 0 ? (
              <path key={index} d={`M${x - 2.5} ${y} h5 M${x} ${y - 2.5} v5`} />
            ) : (
              <circle key={index} cx={x} cy={y} r="0.9" fill="currentColor" stroke="none" />
            ),
          )}
        </g>

        {/* Compass rose over the eastern node. */}
        <g className="terra-rose" transform="translate(1230 690)">
          <circle r="54" />
          <circle r="38" />
          <circle r="6" />
          {Array.from({ length: 8 }, (_, index) => {
            const theta = (index / 8) * Math.PI * 2;
            const outer = index % 2 === 0 ? 92 : 68;
            return (
              <path
                key={index}
                d={`M${Math.cos(theta) * 6} ${Math.sin(theta) * 6} L${Math.cos(theta - 0.09) * 24} ${
                  Math.sin(theta - 0.09) * 24
                } L${Math.cos(theta) * outer} ${Math.sin(theta) * outer} L${
                  Math.cos(theta + 0.09) * 24
                } ${Math.sin(theta + 0.09) * 24} Z`}
              />
            );
          })}
        </g>
      </g>
    </svg>
  );
}
