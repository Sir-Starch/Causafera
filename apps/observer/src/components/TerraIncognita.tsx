/**
 * The chart sheet.
 *
 * A black outline map drawn beneath the whole application: coastlines, engraved water lining,
 * interior contour rings, a graticule, rhumb lines from two compass nodes, soundings, a
 * compass rose, and coasts that break into survey dashes where the survey was never closed.
 *
 * Every line is chart furniture. Nothing here encodes simulation state, and nothing here is
 * random: the coastlines come from fixed sums of sinusoids, so the sheet is identical in every
 * session and can never be mistaken for data (INV-022, INV-039).
 *
 * It is painted once into a canvas rather than kept as live SVG. A hundred vector paths behind
 * a radial mask are re-rasterised on every repaint, and a translucent workspace scrolling over
 * them means a repaint per frame — ruinous on a build with layer compositing turned off. A
 * bitmap of the same drawing costs a blit.
 */

import { useCallback, useEffect, useRef } from "react";

const WIDTH = 1600;
const HEIGHT = 1000;
// Enough samples for the highest harmonic; more only costs paint time.
const SAMPLES = 168;

type Point = [number, number];

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

/** Layer weights. These were CSS opacities when the sheet was SVG; now they are draw values. */
const WEIGHT = {
  graticule: 0.04,
  rhumbs: 0.026,
  lining: 0.05,
  contours: 0.065,
  coast: 0.115,
  coastOpen: 0.08,
  hatch: 0.05,
  soundings: 0.065,
  rose: 0.075,
  grain: 0.022,
};

const SHEET_INK = "#d6e4ea";

function radius(land: Land, turn: number, scale: number): number {
  const theta = turn * Math.PI * 2;
  let value = 1;
  for (const [amplitude, frequency, phase] of land.harmonics) {
    value += amplitude * Math.sin(frequency * theta + phase);
  }
  return land.radius * scale * value;
}

function point(land: Land, turn: number, scale: number, swell = 0): Point {
  const theta = turn * Math.PI * 2;
  const r = radius(land, turn, scale) + swell;
  return [land.cx + r * Math.cos(theta) * land.stretch, land.cy + r * Math.sin(theta)];
}

function ring(land: Land, scale: number, swell = 0): Point[] {
  const points: Point[] = [];
  for (let index = 0; index <= SAMPLES; index += 1) {
    points.push(point(land, index / SAMPLES, scale, swell));
  }
  return points;
}

function inUnsurveyed(land: Land, turn: number): boolean {
  return land.unsurveyed.some(([start, end]) => turn >= start && turn <= end);
}

/** The coastline split into the arcs the survey closed and the arcs it did not. */
function coastArcs(land: Land): { surveyed: Point[][]; unsurveyed: Point[][] } {
  const surveyed: Point[][] = [];
  const unsurveyed: Point[][] = [];
  let run: Point[] = [];
  let runUnsurveyed = inUnsurveyed(land, 0);

  const flush = () => {
    if (run.length > 1) (runUnsurveyed ? unsurveyed : surveyed).push(run);
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

/** Depth soundings on a fixed lattice, skipping anything that falls on land. */
function soundings(): Point[] {
  const marks: Point[] = [];
  for (let x = 90; x < WIDTH; x += 118) {
    for (let y = 70; y < HEIGHT; y += 104) {
      const px = x + (((x * 7 + y * 13) % 37) - 18);
      const py = y + (((x * 11 + y * 5) % 31) - 15);
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

function stroke(context: CanvasRenderingContext2D, points: Point[], close: boolean): void {
  if (points.length < 2) return;
  context.beginPath();
  points.forEach(([x, y], index) => {
    if (index === 0) context.moveTo(x, y);
    else context.lineTo(x, y);
  });
  if (close) context.closePath();
  context.stroke();
}

/** Paper fibre, generated once into a tile. Deterministic, like everything else here. */
function grainTile(): HTMLCanvasElement {
  const size = 128;
  const tile = document.createElement("canvas");
  tile.width = size;
  tile.height = size;
  const context = tile.getContext("2d");
  if (context === null) return tile;
  const image = context.createImageData(size, size);
  let state = 0x9e3779b9;
  for (let index = 0; index < size * size; index += 1) {
    state = (state ^ (state << 13)) >>> 0;
    state = (state ^ (state >>> 17)) >>> 0;
    state = (state ^ (state << 5)) >>> 0;
    const offset = index * 4;
    image.data[offset] = 255;
    image.data[offset + 1] = 255;
    image.data[offset + 2] = 255;
    image.data[offset + 3] = (state & 0xff) >> 2;
  }
  context.putImageData(image, 0, 0);
  return tile;
}

let cachedGrain: HTMLCanvasElement | undefined;

export function TerraIncognita() {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  const paint = useCallback(() => {
    const canvas = canvasRef.current;
    if (canvas === null) return;
    const width = window.innerWidth;
    const height = window.innerHeight;
    if (width === 0 || height === 0) return;
    const ratio = Math.min(window.devicePixelRatio || 1, 2);
    canvas.width = Math.round(width * ratio);
    canvas.height = Math.round(height * ratio);
    const context = canvas.getContext("2d");
    if (context === null) return;

    context.setTransform(ratio, 0, 0, ratio, 0, 0);
    context.clearRect(0, 0, width, height);

    // Cover, matching the SVG `xMidYMid slice` this replaced.
    const scale = Math.max(width / WIDTH, height / HEIGHT);
    context.save();
    context.translate((width - WIDTH * scale) / 2, (height - HEIGHT * scale) / 2);
    context.scale(scale, scale);
    context.lineJoin = "round";
    context.strokeStyle = SHEET_INK;

    const line = (weight: number, lineWidth: number) => {
      context.globalAlpha = weight;
      context.lineWidth = lineWidth / scale;
    };

    // Graticule: meridians and parallels bowed to suggest a projection.
    line(WEIGHT.graticule, 0.7);
    for (let index = 0; index <= 10; index += 1) {
      const x = (index / 10) * WIDTH;
      const bow = (x - WIDTH / 2) * 0.06;
      context.beginPath();
      context.moveTo(x - bow, 0);
      context.quadraticCurveTo(x + bow, HEIGHT / 2, x - bow, HEIGHT);
      context.stroke();
    }
    for (let index = 0; index <= 6; index += 1) {
      const y = (index / 6) * HEIGHT;
      const bow = (y - HEIGHT / 2) * 0.05;
      context.beginPath();
      context.moveTo(0, y + bow);
      context.quadraticCurveTo(WIDTH / 2, y - bow, WIDTH, y + bow);
      context.stroke();
    }

    // Portolan rhumb lines from two compass nodes.
    line(WEIGHT.rhumbs, 0.5);
    for (const [cx, cy, length] of [
      [1230, 690, 1500],
      [470, 430, 1200],
    ] as const) {
      for (let index = 0; index < 16; index += 1) {
        const theta = (index / 16) * Math.PI * 2;
        context.beginPath();
        context.moveTo(cx, cy);
        context.lineTo(cx + Math.cos(theta) * length, cy + Math.sin(theta) * length);
        context.stroke();
      }
    }

    for (const land of LANDS) {
      // Engraved water lining hugging every coast, thinning outwards.
      for (let index = 0; index < land.lining; index += 1) {
        line(WEIGHT.lining * (1 - index * 0.22), 0.7);
        stroke(context, ring(land, 1, 13 + index * (15 + index * 5)), true);
      }
      line(WEIGHT.contours, 0.7);
      for (let index = 0; index < land.contours; index += 1) {
        stroke(context, ring(land, 1 - (index + 1) * (0.62 / (land.contours + 1))), true);
      }
      const coast = coastArcs(land);
      line(WEIGHT.coast, 1.1);
      for (const arc of coast.surveyed) stroke(context, arc, false);
      // Where the coast was never closed, the line breaks into survey dashes.
      line(WEIGHT.coastOpen, 1.1);
      context.setLineDash([7 / scale, 9 / scale]);
      for (const arc of coast.unsurveyed) stroke(context, arc, false);
      context.setLineDash([]);
    }

    // Unsurveyed ground reads as hatching, the same mark the interface uses elsewhere.
    context.globalAlpha = WEIGHT.hatch;
    context.lineWidth = 0.6 / scale;
    for (const corner of [
      [WIDTH * 0.62, 0, WIDTH, 0, WIDTH, HEIGHT * 0.3],
      [0, HEIGHT, 0, HEIGHT * 0.72, WIDTH * 0.14, HEIGHT],
    ]) {
      context.save();
      context.beginPath();
      context.moveTo(corner[0]!, corner[1]!);
      context.lineTo(corner[2]!, corner[3]!);
      context.lineTo(corner[4]!, corner[5]!);
      context.closePath();
      context.clip();
      for (let offset = -HEIGHT; offset < WIDTH + HEIGHT; offset += 9) {
        context.beginPath();
        context.moveTo(offset, 0);
        context.lineTo(offset + HEIGHT, HEIGHT);
        context.stroke();
      }
      context.restore();
    }

    // Soundings: crosses and dots.
    line(WEIGHT.soundings, 0.7);
    context.fillStyle = SHEET_INK;
    soundings().forEach(([x, y], index) => {
      if (index % 5 === 0) {
        context.beginPath();
        context.moveTo(x - 2.5, y);
        context.lineTo(x + 2.5, y);
        context.moveTo(x, y - 2.5);
        context.lineTo(x, y + 2.5);
        context.stroke();
      } else {
        context.beginPath();
        context.arc(x, y, 0.9, 0, Math.PI * 2);
        context.fill();
      }
    });

    // Compass rose over the eastern node.
    line(WEIGHT.rose, 0.7);
    context.save();
    context.translate(1230, 690);
    for (const r of [54, 38, 6]) {
      context.beginPath();
      context.arc(0, 0, r, 0, Math.PI * 2);
      context.stroke();
    }
    for (let index = 0; index < 8; index += 1) {
      const theta = (index / 8) * Math.PI * 2;
      const outer = index % 2 === 0 ? 92 : 68;
      context.beginPath();
      context.moveTo(Math.cos(theta) * 6, Math.sin(theta) * 6);
      context.lineTo(Math.cos(theta - 0.09) * 24, Math.sin(theta - 0.09) * 24);
      context.lineTo(Math.cos(theta) * outer, Math.sin(theta) * outer);
      context.lineTo(Math.cos(theta + 0.09) * 24, Math.sin(theta + 0.09) * 24);
      context.closePath();
      context.stroke();
    }
    context.restore();
    context.restore();

    // The sheet fades towards its edges, the way a lit chart table does.
    context.globalAlpha = 1;
    context.globalCompositeOperation = "destination-in";
    const fade = context.createRadialGradient(
      width * 0.38,
      height * 0.3,
      0,
      width * 0.38,
      height * 0.3,
      Math.max(width, height) * 0.95,
    );
    fade.addColorStop(0, "rgba(255,255,255,0.85)");
    fade.addColorStop(0.62, "rgba(255,255,255,0.45)");
    fade.addColorStop(1, "rgba(255,255,255,0.08)");
    context.fillStyle = fade;
    context.fillRect(0, 0, width, height);
    context.globalCompositeOperation = "source-over";

    // Paper fibre over the whole sheet.
    cachedGrain ??= grainTile();
    const grain = context.createPattern(cachedGrain, "repeat");
    if (grain !== null) {
      context.globalAlpha = WEIGHT.grain;
      context.fillStyle = grain;
      context.fillRect(0, 0, width, height);
      context.globalAlpha = 1;
    }
  }, []);

  useEffect(() => {
    paint();
    let timer: number | undefined;
    const onResize = () => {
      if (timer !== undefined) window.clearTimeout(timer);
      timer = window.setTimeout(paint, 160);
    };
    window.addEventListener("resize", onResize);
    return () => {
      window.removeEventListener("resize", onResize);
      if (timer !== undefined) window.clearTimeout(timer);
    };
  }, [paint]);

  return <canvas ref={canvasRef} className="terra-sheet" aria-hidden="true" />;
}
