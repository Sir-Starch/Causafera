/**
 * The chart profile.
 *
 * Active chunks are drawn as a register of stacked bands — relief, mana, population, causal
 * activity — in chart coordinate order. This is a profile, not a map: chunk coordinates are
 * chart-qualified lattice addresses and adjacency here is ordering, not measured distance
 * (INV-036). The datum line is the only geographic constant drawn, and it is a real zero.
 *
 * One visual variable per quantity: vertical extent is elevation, fill is mana on a single
 * sequential hue, bar length is population, tick marks are causal events, and the pips at
 * the column head are the resolution level.
 */

import { useCallback, useState } from "react";

import type { Atlas, ChunkRecord } from "../observer/models";
import { CanvasSurface, MONO_FONT, MONO_FONT_SM, readPalette, type Frame } from "./canvas";
import { linearScale, niceTicks, normalise } from "./scale";
import { withAlpha } from "./ChartRecorder";

const AXIS_WIDTH = 52;
const RIGHT_PAD = 12;
const TOP_PAD = 14;
const COLUMN_GAP = 8;
const MAX_COLUMN = 132;

const REGISTERS = {
  relief: 168,
  mana: 26,
  population: 20,
  activity: 22,
  labels: 26,
} as const;

export const TRANSECT_HEIGHT =
  TOP_PAD +
  REGISTERS.relief +
  REGISTERS.mana +
  REGISTERS.population +
  REGISTERS.activity +
  REGISTERS.labels;

export interface TransectLabels {
  relief: string;
  mana: string;
  population: string;
  activity: string;
  datum: string;
}

interface TransectProps {
  atlas: Atlas;
  selectedKey?: string;
  onSelect(chunk: ChunkRecord): void;
  labels: TransectLabels;
  ariaLabel: string;
}

interface Geometry {
  columns: { chunk: ChunkRecord; x: number; width: number }[];
}

function geometry(atlas: Atlas, frame: Frame): Geometry {
  const usable = frame.width - AXIS_WIDTH - RIGHT_PAD;
  const count = Math.max(1, atlas.chunks.length);
  const raw = (usable - COLUMN_GAP * (count - 1)) / count;
  const width = Math.max(24, Math.min(MAX_COLUMN, raw));
  const total = width * count + COLUMN_GAP * (count - 1);
  const start = AXIS_WIDTH + Math.max(0, (usable - total) / 2);
  return {
    columns: atlas.chunks.map((chunk, index) => ({
      chunk,
      x: start + index * (width + COLUMN_GAP),
      width,
    })),
  };
}

export function Transect({ atlas, selectedKey, onSelect, labels, ariaLabel }: TransectProps) {
  const [hovered, setHovered] = useState<{ chunk: ChunkRecord; x: number; width: number }>();

  const draw = useCallback(
    (context: CanvasRenderingContext2D, frame: Frame) => {
      if (atlas.chunks.length === 0) return;
      const palette = readPalette();
      const { columns } = geometry(atlas, frame);

      const reliefTop = TOP_PAD;
      const reliefBottom = reliefTop + REGISTERS.relief;
      const manaTop = reliefBottom;
      const populationTop = manaTop + REGISTERS.mana;
      const activityTop = populationTop + REGISTERS.population;
      const labelTop = activityTop + REGISTERS.activity;

      const elevationPad = Math.max(1, (atlas.elevationMax - atlas.elevationMin) * 0.08);
      const elevation = linearScale(
        [atlas.elevationMin - elevationPad, atlas.elevationMax + elevationPad],
        [reliefBottom - 6, reliefTop + 6],
      );

      // Elevation axis in metres.
      context.font = MONO_FONT;
      context.textAlign = "right";
      context.textBaseline = "middle";
      for (const tick of niceTicks(atlas.elevationMin, atlas.elevationMax, 4)) {
        const y = Math.round(elevation(tick)) + 0.5;
        context.strokeStyle = palette.ruleGhost!;
        context.lineWidth = 1;
        context.beginPath();
        context.moveTo(AXIS_WIDTH - 6, y);
        context.lineTo(frame.width - RIGHT_PAD, y);
        context.stroke();
        context.fillStyle = palette.inkGhost!;
        context.fillText((tick / 1000).toFixed(0), AXIS_WIDTH - 10, y);
      }

      // Datum: real zero elevation.
      if (atlas.elevationMin < 0 && atlas.elevationMax > 0) {
        const y = Math.round(elevation(0)) + 0.5;
        context.strokeStyle = withAlpha(palette.beacon!, 0.34);
        context.setLineDash([5, 4]);
        context.lineWidth = 1;
        context.beginPath();
        context.moveTo(AXIS_WIDTH - 6, y);
        context.lineTo(frame.width - RIGHT_PAD, y);
        context.stroke();
        context.setLineDash([]);
        context.font = MONO_FONT_SM;
        context.textAlign = "left";
        context.fillStyle = withAlpha(palette.beacon!, 0.7);
        context.fillText(labels.datum, AXIS_WIDTH + 2, y - 6);
      }

      // Register separators and their left-margin labels.
      context.textAlign = "right";
      context.textBaseline = "middle";
      context.font = MONO_FONT_SM;
      const registerRows: [number, number, string][] = [
        [manaTop, REGISTERS.mana, labels.mana],
        [populationTop, REGISTERS.population, labels.population],
        [activityTop, REGISTERS.activity, labels.activity],
      ];
      for (const [top, height, text] of registerRows) {
        context.strokeStyle = palette.ruleFaint!;
        context.beginPath();
        context.moveTo(AXIS_WIDTH - 6, Math.round(top) + 0.5);
        context.lineTo(frame.width - RIGHT_PAD, Math.round(top) + 0.5);
        context.stroke();
        context.fillStyle = palette.inkGhost!;
        context.fillText(text, AXIS_WIDTH - 10, top + height / 2);
      }
      context.fillStyle = palette.inkGhost!;
      context.fillText(labels.relief, AXIS_WIDTH - 10, reliefTop + 8);

      for (const { chunk, x, width } of columns) {
        const selected = chunk.key === selectedKey;
        const active = selected || hovered?.chunk.key === chunk.key;

        // Relief band: the measured elevation range of the chunk.
        const top = elevation(chunk.maximumElevationMm);
        const bottom = elevation(chunk.minimumElevationMm);
        const gradient = context.createLinearGradient(0, top, 0, bottom);
        gradient.addColorStop(0, withAlpha(palette.inkDim!, 0.3));
        gradient.addColorStop(1, withAlpha(palette.inkDim!, 0.08));
        context.fillStyle = gradient;
        context.fillRect(x, top, width, Math.max(2, bottom - top));

        context.strokeStyle = active ? palette.beacon! : withAlpha(palette.inkDim!, 0.55);
        context.lineWidth = active ? 1.5 : 1;
        context.beginPath();
        context.moveTo(x, Math.round(top) + 0.5);
        context.lineTo(x + width, Math.round(top) + 0.5);
        context.stroke();
        context.strokeStyle = withAlpha(palette.inkDim!, 0.35);
        context.lineWidth = 1;
        context.beginPath();
        context.moveTo(x, Math.round(bottom) + 0.5);
        context.lineTo(x + width, Math.round(bottom) + 0.5);
        context.stroke();

        // Resolution level: discrete pips at the column head.
        const pipY = reliefTop + 5;
        for (let level = 0; level < chunk.resolutionLevel; level += 1) {
          context.fillStyle = palette.resolution!;
          context.fillRect(x + 1 + level * 5, pipY, 3, 6);
        }
        if (chunk.resolutionLevel === 0) {
          context.strokeStyle = withAlpha(palette.resolution!, 0.4);
          context.strokeRect(x + 1.5, pipY + 0.5, 3, 5);
        }

        // Mana register: one sequential hue, magnitude by fill.
        const manaFraction = normalise(chunk.manaTotal, 0, atlas.manaMax);
        context.fillStyle = withAlpha(palette.ramp700!, 0.35);
        context.fillRect(x, manaTop + 5, width, REGISTERS.mana - 10);
        context.fillStyle = rampColor(manaFraction, palette);
        context.fillRect(x, manaTop + 5, width * manaFraction, REGISTERS.mana - 10);

        // Population register.
        const populationFraction = normalise(chunk.populationTotal, 0, atlas.populationMax);
        context.fillStyle = withAlpha(palette.life!, 0.16);
        context.fillRect(x, populationTop + 5, width, REGISTERS.population - 10);
        if (chunk.populationTotal > 0) {
          context.fillStyle = palette.life!;
          context.fillRect(
            x,
            populationTop + 5,
            Math.max(2, width * populationFraction),
            REGISTERS.population - 10,
          );
        }

        // Activity register: causal events as ticks, surface transitions as diamonds.
        const ticks = Math.min(chunk.causalEventCount, Math.floor(width / 4));
        context.strokeStyle = palette.physical!;
        context.lineWidth = 1;
        for (let index = 0; index < ticks; index += 1) {
          const tx = Math.round(x + 2 + index * 4) + 0.5;
          context.beginPath();
          context.moveTo(tx, activityTop + 6);
          context.lineTo(tx, activityTop + 14);
          context.stroke();
        }
        if (chunk.transitions > 0) {
          context.fillStyle = palette.mana!;
          for (let index = 0; index < Math.min(chunk.transitions, Math.floor(width / 8)); index += 1) {
            const cx = x + 4 + index * 8;
            const cy = activityTop + REGISTERS.activity - 5;
            context.beginPath();
            context.moveTo(cx, cy - 3);
            context.lineTo(cx + 3, cy);
            context.lineTo(cx, cy + 3);
            context.lineTo(cx - 3, cy);
            context.closePath();
            context.fill();
          }
        }

        // Column label.
        context.font = MONO_FONT;
        context.textAlign = "center";
        context.textBaseline = "top";
        context.fillStyle = active ? palette.ink! : palette.inkFaint!;
        context.fillText(
          `${chunk.chunkX} ${chunk.chunkY} ${chunk.chunkZ}`,
          x + width / 2,
          labelTop + 6,
        );

        // Selection reads as a coordinate lock, not a filled highlight.
        if (selected) {
          context.strokeStyle = palette.beacon!;
          context.lineWidth = 1;
          const armX = Math.min(10, width / 3);
          const boxTop = reliefTop + 1;
          const boxBottom = activityTop + REGISTERS.activity - 1;
          const corners: [number, number, number, number][] = [
            [x, boxTop, armX, 8],
            [x + width, boxTop, -armX, 8],
            [x, boxBottom, armX, -8],
            [x + width, boxBottom, -armX, -8],
          ];
          for (const [cx, cy, dx, dy] of corners) {
            context.beginPath();
            context.moveTo(cx + dx, cy + 0.5);
            context.lineTo(cx + (dx > 0 ? 0.5 : -0.5), cy + 0.5);
            context.lineTo(cx + (dx > 0 ? 0.5 : -0.5), cy + dy);
            context.stroke();
          }
        }
      }
    },
    [atlas, hovered, labels, selectedKey],
  );

  const locate = useCallback(
    (point: { x: number; y: number; frame: Frame } | undefined) => {
      if (point === undefined) return undefined;
      const { columns } = geometry(atlas, point.frame);
      return columns.find(
        (column) => point.x >= column.x && point.x <= column.x + column.width,
      );
    },
    [atlas],
  );

  return (
    <div className="transect">
      <CanvasSurface
        height={TRANSECT_HEIGHT}
        draw={draw}
        label={ariaLabel}
        onProbe={(point) => setHovered(locate(point))}
        onActivate={(point) => {
          const column = locate(point);
          if (column !== undefined) onSelect(column.chunk);
        }}
      />
    </div>
  );
}

/** Sequential ramp lookup: light means little, dark means much, one hue throughout. */
function rampColor(fraction: number, palette: Record<string, string>): string {
  if (fraction > 0.72) return palette.ramp100!;
  if (fraction > 0.45) return palette.ramp300!;
  if (fraction > 0.18) return palette.ramp500!;
  return palette.ramp700!;
}
