/**
 * The chart table.
 *
 * A pannable, zoomable plan view of one chart's chunk lattice. It draws what the lenses hand
 * it — a field, symbols, cell marks, vectors, isolines — and knows nothing about what any of
 * those mean. Ground the observer has not received is drawn as unsurveyed hatching rather
 * than as empty space, because on this instrument "not observed" is a finding.
 *
 * Level of detail follows legibility: a field of colour when chunks are small, chunk glyphs
 * and labels at reading size, and the 32³ cell lattice once a cell is large enough to carry a
 * mark. Where a lens has only a chunk aggregate, the cell lattice is hatched over to say so.
 *
 * Culling is by viewport, so the renderer costs a screenful regardless of how large a chart
 * the runtime eventually projects.
 */

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { MONO_FONT, MONO_FONT_SM, hatchPattern, readPalette } from "../viz/canvas";
import { withAlpha } from "../viz/ChartRecorder";
import type { ChunkRecord } from "../observer/models";
import { fieldIntensity, type Lens, type LensContext, type LensLayers } from "./lens";
import { lensLayers } from "./lenses";
import {
  CELLS_PER_EDGE,
  CHUNK_UNITS,
  chunkAt,
  cellAt,
  chunkPixels,
  detailFor,
  extentOf,
  fitViewport,
  graticuleStep,
  toScreen,
  toWorld,
  visibleChunks,
  worldOfChunk,
  zoomAt,
  type Screen,
  type Viewport,
} from "./projection";

export interface MapSelection {
  chunkKey?: string;
  cell?: { chunkKey: string; x: number; y: number };
}

export interface ChartMapLabels {
  unsurveyed: string;
  chunkAggregate: string;
  scale: string;
  north: string;
}

interface ChartMapProps {
  context: LensContext;
  primary?: Lens;
  overlays: Lens[];
  selection: MapSelection;
  onSelect(selection: MapSelection): void;
  onHover(hover: HoverReading | undefined): void;
  labels: ChartMapLabels;
  ariaLabel: string;
}

export interface HoverReading {
  chunkX: number;
  chunkY: number;
  chunk?: ChunkRecord;
  cell?: { x: number; y: number };
  value?: string;
}

export function ChartMap({
  context,
  primary,
  overlays,
  selection,
  onSelect,
  onHover,
  labels,
  ariaLabel,
}: ChartMapProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [screen, setScreen] = useState<Screen>({ width: 0, height: 0 });
  const [viewport, setViewport] = useState<Viewport>({ x: 0, y: 0, scale: 1 });
  const [hover, setHover] = useState<{ chunkX: number; chunkY: number } | undefined>();
  const dragRef = useRef<{ x: number; y: number; moved: boolean } | undefined>(undefined);
  const publishedRef = useRef<HoverReading | undefined>(undefined);
  const fittedRef = useRef(false);

  const chunkIndex = useMemo(() => {
    const index = new Map<string, ChunkRecord>();
    for (const chunk of context.atlas.chunks) index.set(`${chunk.chunkX}:${chunk.chunkY}`, chunk);
    return index;
  }, [context.atlas]);

  const extent = useMemo(() => extentOf(context.atlas.chunks), [context.atlas]);

  const primaryLayers = useMemo<LensLayers>(
    () => (primary === undefined ? {} : lensLayers(primary, context)),
    [primary, context],
  );
  const overlayLayers = useMemo(
    () => overlays.map((lens) => ({ lens, layers: lensLayers(lens, context) })),
    [overlays, context],
  );

  /* ------------------------------------------------------------ sizing -- */

  useEffect(() => {
    const host = hostRef.current;
    if (host === null) return undefined;
    const observer = new ResizeObserver(() => {
      setScreen({
        width: Math.max(1, Math.round(host.clientWidth)),
        height: Math.max(1, Math.round(host.clientHeight)),
      });
    });
    observer.observe(host);
    setScreen({
      width: Math.max(1, Math.round(host.clientWidth)),
      height: Math.max(1, Math.round(host.clientHeight)),
    });
    return () => observer.disconnect();
  }, []);

  const fit = useCallback(() => {
    setViewport(fitViewport(extent, screen));
  }, [extent, screen]);

  // Frame the charted extent once, the first time both a chart and a size exist.
  useEffect(() => {
    if (fittedRef.current || extent === undefined || screen.width === 0) return;
    fittedRef.current = true;
    setViewport(fitViewport(extent, screen));
  }, [extent, screen]);

  /* ------------------------------------------------------------ drawing -- */

  const draw = useCallback(
    (canvas: HTMLCanvasElement) => {
      if (screen.width === 0 || screen.height === 0) return;
      const ratio = Math.min(window.devicePixelRatio || 1, 2);
      canvas.width = Math.round(screen.width * ratio);
      canvas.height = Math.round(screen.height * ratio);
      const context2d = canvas.getContext("2d");
      if (context2d === null) return;
      context2d.setTransform(ratio, 0, 0, ratio, 0, 0);
      context2d.clearRect(0, 0, screen.width, screen.height);

      const palette = readPalette();
      const detail = detailFor(viewport.scale);
      const pixels = chunkPixels(viewport.scale);
      const signal = primary === undefined ? palette.inkDim! : palette[primary.signal]!;

      const project = (worldX: number, worldY: number) =>
        toScreen(viewport, screen, worldX, worldY);

      // 1. Unsurveyed ground everywhere; charted chunks are painted over it.
      context2d.fillStyle = hatchPattern(context2d, withAlpha(palette.inkDim!, 0.26), 7, 0.6);
      context2d.fillRect(0, 0, screen.width, screen.height);

      // 2. Chart graticule.
      const step = graticuleStep(viewport.scale);
      const window_ = visibleChunks(viewport, screen);
      context2d.lineWidth = 1;
      context2d.font = MONO_FONT_SM;
      context2d.textBaseline = "top";
      context2d.textAlign = "left";
      for (let cx = Math.ceil(window_.minX / step) * step; cx <= window_.maxX; cx += step) {
        const world = worldOfChunk(cx, 0);
        const x = Math.round(project(world.x - CHUNK_UNITS / 2, 0).x) + 0.5;
        context2d.strokeStyle = cx === 0 ? palette.ruleFaint! : palette.ruleGhost!;
        context2d.beginPath();
        context2d.moveTo(x, 0);
        context2d.lineTo(x, screen.height);
        context2d.stroke();
        if (pixels > 22) {
          context2d.fillStyle = palette.inkGhost!;
          context2d.fillText(`${cx}`, x + 3, 4);
        }
      }
      for (let cy = Math.ceil(window_.minY / step) * step; cy <= window_.maxY; cy += step) {
        const world = worldOfChunk(0, cy);
        const y = Math.round(project(0, world.y - CHUNK_UNITS / 2).y) + 0.5;
        context2d.strokeStyle = cy === 0 ? palette.ruleFaint! : palette.ruleGhost!;
        context2d.beginPath();
        context2d.moveTo(0, y);
        context2d.lineTo(screen.width, y);
        context2d.stroke();
        if (pixels > 22) {
          context2d.fillStyle = palette.inkGhost!;
          context2d.fillText(`${cy}`, 4, y + 3);
        }
      }

      // 3. Charted chunks.
      const field = primaryLayers.field;
      const drawn: { chunk: ChunkRecord; x: number; y: number; size: number }[] = [];
      for (let cy = window_.minY; cy <= window_.maxY; cy += 1) {
        for (let cx = window_.minX; cx <= window_.maxX; cx += 1) {
          const chunk = chunkIndex.get(`${cx}:${cy}`);
          if (chunk === undefined) continue;
          const world = worldOfChunk(cx, cy);
          const topLeft = project(world.x - CHUNK_UNITS / 2, world.y - CHUNK_UNITS / 2);
          const size = CHUNK_UNITS * viewport.scale;
          drawn.push({ chunk, x: topLeft.x, y: topLeft.y, size });

          // Surveyed ground masks the hatching beneath it.
          context2d.fillStyle = palette.paper!;
          context2d.fillRect(topLeft.x, topLeft.y, size, size);

          const value = field?.values.get(chunk.key);
          if (field !== undefined && value !== undefined) {
            context2d.fillStyle = withAlpha(signal, 0.05 + fieldIntensity(field, value) * 0.3);
            context2d.fillRect(topLeft.x, topLeft.y, size, size);
          }

          context2d.strokeStyle = palette.ruleFaint!;
          context2d.lineWidth = 1;
          context2d.strokeRect(
            Math.round(topLeft.x) + 0.5,
            Math.round(topLeft.y) + 0.5,
            Math.round(size) - 1,
            Math.round(size) - 1,
          );
        }
      }

      // 4. Cell lattice, once a cell can carry a mark.
      if (detail === "cell") {
        const cellSize = (CHUNK_UNITS * viewport.scale) / CELLS_PER_EDGE;
        context2d.strokeStyle = palette.ruleGhost!;
        context2d.lineWidth = 1;
        for (const { x, y, size } of drawn) {
          context2d.beginPath();
          for (let index = 1; index < CELLS_PER_EDGE; index += 1) {
            const offset = index * cellSize;
            context2d.moveTo(Math.round(x + offset) + 0.5, y);
            context2d.lineTo(Math.round(x + offset) + 0.5, y + size);
            context2d.moveTo(x, Math.round(y + offset) + 0.5);
            context2d.lineTo(x + size, Math.round(y + offset) + 0.5);
          }
          context2d.stroke();
        }
        // A chunk aggregate is not a cell measurement, and the hatch says so.
        if (primary !== undefined && primary.cellProjection === "none" && field !== undefined) {
          context2d.fillStyle = hatchPattern(context2d, withAlpha(signal, 0.16), 10, 0.7);
          for (const { x, y, size } of drawn) context2d.fillRect(x, y, size, size);
        }
      }

      // 5. Isolines, clipped to the charted extent. An interpolation must not paint knowledge
      // over ground the observer never received.
      context2d.save();
      if (extent !== undefined) {
        const margin = CHUNK_UNITS * 0.55;
        const topLeft = worldOfChunk(extent.minX, extent.maxY);
        const bottomRight = worldOfChunk(extent.maxX, extent.minY);
        const a = project(topLeft.x - CHUNK_UNITS / 2 - margin, topLeft.y - CHUNK_UNITS / 2 - margin);
        const b = project(
          bottomRight.x + CHUNK_UNITS / 2 + margin,
          bottomRight.y + CHUNK_UNITS / 2 + margin,
        );
        context2d.beginPath();
        context2d.rect(a.x, a.y, b.x - a.x, b.y - a.y);
        context2d.clip();
      }
      for (const { lens, layers } of overlayLayers) {
        if (layers.isolines === undefined) continue;
        const colour = palette[lens.signal]!;
        for (const line of layers.isolines) {
          if (line.points.length < 2) continue;
          context2d.strokeStyle = withAlpha(colour, 0.24 + line.level * 0.4);
          context2d.lineWidth = line.level > 0.66 ? 1.3 : 0.9;
          context2d.beginPath();
          line.points.forEach(([wx, wy], index) => {
            const point = project(wx, wy);
            if (index === 0) context2d.moveTo(point.x, point.y);
            else context2d.lineTo(point.x, point.y);
          });
          context2d.stroke();
        }
      }
      context2d.restore();

      // 6. Vectors.
      for (const { lens, layers } of overlayLayers) {
        if (layers.vectors === undefined) continue;
        const colour = palette[lens.signal]!;
        for (const vector of layers.vectors) {
          const from = project(vector.fromX, vector.fromY);
          const to = project(vector.toX, vector.toY);
          const dx = to.x - from.x;
          const dy = to.y - from.y;
          const length = Math.hypot(dx, dy);
          if (length < 6) continue;
          const inset = Math.min(length * 0.28, pixels * 0.3);
          const ux = dx / length;
          const uy = dy / length;
          const ax = from.x + ux * inset;
          const ay = from.y + uy * inset;
          const bx = to.x - ux * inset;
          const by = to.y - uy * inset;
          context2d.strokeStyle = withAlpha(colour, 0.35 + vector.weight * 0.5);
          context2d.lineWidth = 1 + vector.weight * 1.6;
          context2d.beginPath();
          context2d.moveTo(ax, ay);
          context2d.lineTo(bx, by);
          context2d.stroke();
          const head = 5 + vector.weight * 5;
          context2d.beginPath();
          context2d.moveTo(bx, by);
          context2d.lineTo(bx - ux * head - uy * head * 0.45, by - uy * head + ux * head * 0.45);
          context2d.moveTo(bx, by);
          context2d.lineTo(bx - ux * head + uy * head * 0.45, by - uy * head - ux * head * 0.45);
          context2d.stroke();
        }
      }

      // 7. Cell marks at real lattice positions.
      if (detail === "cell") {
        const cellSize = (CHUNK_UNITS * viewport.scale) / CELLS_PER_EDGE;
        for (const { lens, layers } of overlayLayers.concat(
          primary === undefined ? [] : [{ lens: primary, layers: primaryLayers }],
        )) {
          if (layers.cells === undefined) continue;
          const colour = palette[lens.signal]!;
          for (const mark of layers.cells) {
            const world = worldOfChunk(mark.chunkX, mark.chunkY);
            const origin = project(world.x - CHUNK_UNITS / 2, world.y - CHUNK_UNITS / 2);
            const cx = origin.x + (mark.cellX + 0.5) * cellSize;
            const cy = origin.y + (CELLS_PER_EDGE - 1 - mark.cellY + 0.5) * cellSize;
            const radius = Math.max(3.5, Math.min(cellSize * 0.42, 11));
            drawShape(context2d, mark.shape, cx, cy, radius, colour, mark.intensity, palette.paper!);
          }
        }
      }

      // 8. Symbols. A proportional circle owns the chunk centre; fixed marks queue along the
      // foot of the chunk so several overlays never stack on the same point.
      if (detail !== "field") {
        const markLayers = overlayLayers.filter(
          ({ layers }) =>
            layers.symbols !== undefined &&
            layers.symbols.some((symbol) => (symbol.shape ?? "circle") !== "circle"),
        );
        for (const { lens, layers } of overlayLayers) {
          if (layers.symbols === undefined) continue;
          if (detail === "cell" && layers.cells !== undefined && layers.cells.length > 0) continue;
          const colour = palette[lens.signal]!;
          const markIndex = markLayers.findIndex((entry) => entry.lens.id === lens.id);
          for (const symbol of layers.symbols) {
            const world = worldOfChunk(symbol.chunkX, symbol.chunkY);
            const centre = project(world.x, world.y);
            const shape = symbol.shape ?? "circle";
            if (shape === "circle") {
              const radius = Math.max(3, Math.min(symbol.weight * pixels * 0.2, 46));
              drawShape(context2d, shape, centre.x, centre.y, radius, colour, 1, palette.paper!);
              if (detail === "cell") {
                context2d.font = MONO_FONT;
                context2d.textAlign = "center";
                context2d.textBaseline = "middle";
                context2d.fillStyle = palette.ink!;
                context2d.fillText(symbol.label, centre.x, centre.y + radius + 9);
              }
              continue;
            }
            const radius = Math.max(3.5, Math.min(pixels * 0.055, 9));
            const spacing = radius * 2.8;
            const offset = (markIndex - (markLayers.length - 1) / 2) * spacing;
            drawShape(
              context2d,
              shape,
              centre.x + offset,
              centre.y + pixels * 0.3,
              radius,
              colour,
              1,
              palette.paper!,
            );
          }
        }
      }

      // 9. Chunk labels and the primary value.
      if (detail !== "field") {
        context2d.textAlign = "center";
        for (const { chunk, x, y, size } of drawn) {
          context2d.font = MONO_FONT;
          context2d.textBaseline = "top";
          context2d.fillStyle = palette.inkFaint!;
          context2d.fillText(`${chunk.chunkX}, ${chunk.chunkY}`, x + size / 2, y + 6);
          const value = field?.values.get(chunk.key);
          if (field !== undefined && value !== undefined && size > 78) {
            context2d.font = `${Math.min(19, Math.max(11, size * 0.13))}px ui-monospace, monospace`;
            context2d.fillStyle = palette.ink!;
            context2d.fillText(field.format(value), x + size / 2, y + 20);
          }
        }
      }

      // 10. Hover and selection read as survey marks, never as a wash.
      for (const { chunk, x, y, size } of drawn) {
        const isSelected = chunk.key === selection.chunkKey;
        const isHovered = hover?.chunkX === chunk.chunkX && hover.chunkY === chunk.chunkY;
        if (!isSelected && !isHovered) continue;
        context2d.strokeStyle = isSelected ? palette.mark! : palette.ruleBright!;
        context2d.lineWidth = 1;
        if (isSelected) {
          const arm = Math.min(size * 0.28, 22);
          for (const [ox, oy, sx, sy] of [
            [x, y, 1, 1],
            [x + size, y, -1, 1],
            [x, y + size, 1, -1],
            [x + size, y + size, -1, -1],
          ] as const) {
            context2d.beginPath();
            context2d.moveTo(ox + sx * arm, oy + 0.5 * sy);
            context2d.lineTo(ox, oy);
            context2d.lineTo(ox + 0.5 * sx, oy + sy * arm);
            context2d.stroke();
          }
        } else {
          context2d.setLineDash([3, 3]);
          context2d.strokeRect(
            Math.round(x) + 0.5,
            Math.round(y) + 0.5,
            Math.round(size) - 1,
            Math.round(size) - 1,
          );
          context2d.setLineDash([]);
        }
      }

      // 11. The charted boundary: where the survey stops.
      if (extent !== undefined) {
        const topLeft = worldOfChunk(extent.minX, extent.maxY);
        const bottomRight = worldOfChunk(extent.maxX, extent.minY);
        const a = project(topLeft.x - CHUNK_UNITS / 2, topLeft.y - CHUNK_UNITS / 2);
        const b = project(bottomRight.x + CHUNK_UNITS / 2, bottomRight.y + CHUNK_UNITS / 2);
        context2d.strokeStyle = withAlpha(palette.inkDim!, 0.5);
        context2d.lineWidth = 1;
        context2d.strokeRect(a.x, a.y, b.x - a.x, b.y - a.y);
        // Engraved lining outside the boundary, thinning outwards.
        for (let index = 1; index <= 3; index += 1) {
          const inset = -index * (3 + index * 2);
          context2d.strokeStyle = withAlpha(palette.inkDim!, 0.2 / index);
          context2d.strokeRect(a.x + inset, a.y + inset, b.x - a.x - inset * 2, b.y - a.y - inset * 2);
        }
        context2d.font = MONO_FONT_SM;
        context2d.textAlign = "left";
        context2d.textBaseline = "bottom";
        context2d.fillStyle = palette.inkFaint!;
        const caption = labels.unsurveyed.toUpperCase();
        const captionWidth = context2d.measureText(caption).width;
        const captionX = Math.max(8, Math.min(a.x, screen.width - captionWidth - 8));
        const captionY = Math.max(18, a.y - 8);
        context2d.fillStyle = palette.paper!;
        context2d.fillRect(captionX - 3, captionY - 10, captionWidth + 6, 12);
        context2d.fillStyle = palette.inkFaint!;
        context2d.fillText(caption, captionX, captionY);
      }
    },
    [
      chunkIndex,
      extent,
      hover,
      labels,
      overlayLayers,
      primary,
      primaryLayers,
      screen,
      selection.chunkKey,
      viewport,
    ],
  );

  useEffect(() => {
    const canvas = canvasRef.current;
    if (canvas !== null) draw(canvas);
  }, [draw]);

  /* -------------------------------------------------------- interaction -- */

  const readAt = useCallback(
    (screenX: number, screenY: number): HoverReading | undefined => {
      if (screen.width === 0) return undefined;
      const world = toWorld(viewport, screen, screenX, screenY);
      const position = chunkAt(world.x, world.y);
      const chunk = chunkIndex.get(`${position.chunkX}:${position.chunkY}`);
      const detail = detailFor(viewport.scale);
      const cell = detail === "cell" ? cellAt(world.x, world.y) : undefined;
      const field = primaryLayers.field;
      const raw = chunk === undefined ? undefined : field?.values.get(chunk.key);
      return {
        chunkX: position.chunkX,
        chunkY: position.chunkY,
        chunk,
        cell: cell === undefined ? undefined : { x: cell.cellX, y: cell.cellY },
        value: raw === undefined || field === undefined ? undefined : field.format(raw),
      };
    },
    [chunkIndex, primaryLayers.field, screen, viewport],
  );

  // Wheel must be non-passive to keep the page from scrolling under the map.
  useEffect(() => {
    const host = hostRef.current;
    if (host === null) return undefined;
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      const rect = host.getBoundingClientRect();
      const factor = Math.exp(-event.deltaY * 0.0016);
      setViewport((current) =>
        zoomAt(current, screen, event.clientX - rect.left, event.clientY - rect.top, factor),
      );
    };
    host.addEventListener("wheel", onWheel, { passive: false });
    return () => host.removeEventListener("wheel", onWheel);
  }, [screen]);

  return (
    <div
      ref={hostRef}
      className="chart-map"
      role="application"
      aria-label={ariaLabel}
      tabIndex={0}
      onPointerDown={(event) => {
        (event.target as HTMLElement).setPointerCapture?.(event.pointerId);
        dragRef.current = { x: event.clientX, y: event.clientY, moved: false };
      }}
      onPointerMove={(event) => {
        const rect = event.currentTarget.getBoundingClientRect();
        const drag = dragRef.current;
        if (drag !== undefined) {
          const dx = event.clientX - drag.x;
          const dy = event.clientY - drag.y;
          if (Math.hypot(dx, dy) > 2) drag.moved = true;
          dragRef.current = { x: event.clientX, y: event.clientY, moved: drag.moved };
          setViewport((current) => ({
            ...current,
            x: current.x - dx / current.scale,
            y: current.y - dy / current.scale,
          }));
          return;
        }
        const reading = readAt(event.clientX - rect.left, event.clientY - rect.top);
        // Both of these drive a render and one repaints the whole canvas, so only publish a
        // reading when it actually says something different.
        setHover((current) =>
          reading !== undefined &&
          current?.chunkX === reading.chunkX &&
          current.chunkY === reading.chunkY
            ? current
            : reading === undefined
              ? undefined
              : { chunkX: reading.chunkX, chunkY: reading.chunkY },
        );
        const previous = publishedRef.current;
        if (
          previous?.chunkX !== reading?.chunkX ||
          previous?.chunkY !== reading?.chunkY ||
          previous?.cell?.x !== reading?.cell?.x ||
          previous?.cell?.y !== reading?.cell?.y ||
          previous?.value !== reading?.value
        ) {
          publishedRef.current = reading;
          onHover(reading);
        }
      }}
      onPointerUp={(event) => {
        const drag = dragRef.current;
        dragRef.current = undefined;
        if (drag?.moved === true) return;
        const rect = event.currentTarget.getBoundingClientRect();
        const reading = readAt(event.clientX - rect.left, event.clientY - rect.top);
        if (reading?.chunk === undefined) {
          onSelect({});
          return;
        }
        onSelect({
          chunkKey: reading.chunk.key,
          cell:
            reading.cell === undefined
              ? undefined
              : { chunkKey: reading.chunk.key, x: reading.cell.x, y: reading.cell.y },
        });
      }}
      onPointerLeave={() => {
        dragRef.current = undefined;
        publishedRef.current = undefined;
        setHover(undefined);
        onHover(undefined);
      }}
      onKeyDown={(event) => {
        const pan = event.shiftKey ? 240 : 90;
        if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
          event.preventDefault();
          const direction = event.key === "ArrowLeft" ? -1 : 1;
          setViewport((current) => ({ ...current, x: current.x + (direction * pan) / current.scale }));
        } else if (event.key === "ArrowUp" || event.key === "ArrowDown") {
          event.preventDefault();
          const direction = event.key === "ArrowUp" ? -1 : 1;
          setViewport((current) => ({ ...current, y: current.y + (direction * pan) / current.scale }));
        } else if (event.key === "+" || event.key === "=") {
          setViewport((current) => zoomAt(current, screen, screen.width / 2, screen.height / 2, 1.3));
        } else if (event.key === "-") {
          setViewport((current) => zoomAt(current, screen, screen.width / 2, screen.height / 2, 1 / 1.3));
        } else if (event.key === "0") {
          fit();
        }
      }}
    >
      <canvas ref={canvasRef} />
      <MapControls viewport={viewport} labels={labels} onFit={fit} onZoom={(factor) =>
        setViewport((current) => zoomAt(current, screen, screen.width / 2, screen.height / 2, factor))
      } />
    </div>
  );
}

/* --------------------------------------------------------------- shapes -- */

function drawShape(
  context: CanvasRenderingContext2D,
  shape: string,
  x: number,
  y: number,
  radius: number,
  colour: string,
  intensity: number,
  paper: string,
): void {
  context.lineWidth = 1.4;
  context.strokeStyle = paper;
  context.fillStyle = withAlpha(colour, 0.35 + intensity * 0.5);
  if (shape === "circle") {
    context.fillStyle = withAlpha(colour, 0.16);
    context.beginPath();
    context.arc(x, y, radius, 0, Math.PI * 2);
    context.fill();
    context.strokeStyle = withAlpha(colour, 0.9);
    context.lineWidth = 1.2;
    context.beginPath();
    context.arc(x, y, radius, 0, Math.PI * 2);
    context.stroke();
    return;
  }
  if (shape === "diamond") {
    context.beginPath();
    context.moveTo(x, y - radius);
    context.lineTo(x + radius, y);
    context.lineTo(x, y + radius);
    context.lineTo(x - radius, y);
    context.closePath();
    context.fill();
    context.stroke();
    return;
  }
  if (shape === "square") {
    context.beginPath();
    context.rect(x - radius * 0.8, y - radius * 0.8, radius * 1.6, radius * 1.6);
    context.fill();
    context.stroke();
    return;
  }
  if (shape === "ring") {
    context.strokeStyle = withAlpha(colour, 0.9);
    context.lineWidth = 1.4;
    context.beginPath();
    context.arc(x, y, radius, 0, Math.PI * 2);
    context.stroke();
    return;
  }
  // cross
  context.strokeStyle = withAlpha(colour, 0.85);
  context.lineWidth = 1.2;
  context.beginPath();
  context.moveTo(x - radius, y);
  context.lineTo(x + radius, y);
  context.moveTo(x, y - radius);
  context.lineTo(x, y + radius);
  context.stroke();
}

/* ------------------------------------------------------------- controls -- */

function MapControls({
  viewport,
  labels,
  onFit,
  onZoom,
}: {
  viewport: Viewport;
  labels: ChartMapLabels;
  onFit(): void;
  onZoom(factor: number): void;
}) {
  // A scale bar in chunks: the only spatial unit the chart contract actually defines.
  const targetPixels = 108;
  const rawChunks = Math.max(1, targetPixels / chunkPixels(viewport.scale));
  const magnitude = 10 ** Math.floor(Math.log10(rawChunks));
  const normalised = rawChunks / magnitude;
  const chunks = Math.round((normalised >= 5 ? 5 : normalised >= 2 ? 2 : 1) * magnitude);
  const barPixels = chunks * chunkPixels(viewport.scale);

  return (
    <div className="chart-map__controls">
      <div className="chart-map__rose" aria-hidden="true">
        <svg viewBox="0 0 40 40" width="40" height="40">
          <circle cx="20" cy="20" r="15" fill="none" stroke="currentColor" strokeWidth="0.8" />
          <path d="M20 4 L23 20 L20 36 L17 20 Z" fill="currentColor" opacity="0.75" />
          <path d="M4 20 h32" stroke="currentColor" strokeWidth="0.6" opacity="0.4" />
          <text x="20" y="3" fontSize="6" textAnchor="middle" fill="currentColor">
            {labels.north}
          </text>
        </svg>
      </div>
      <div className="chart-map__scale">
        <span className="chart-map__scale-bar" style={{ width: `${Math.round(barPixels)}px` }} />
        <span className="numeric">
          {chunks} {labels.scale}
        </span>
      </div>
      <div className="chart-map__zoom">
        <button type="button" className="btn btn--icon" onClick={() => onZoom(1.35)} aria-label="+">
          +
        </button>
        <button
          type="button"
          className="btn btn--icon"
          onClick={() => onZoom(1 / 1.35)}
          aria-label="−"
        >
          −
        </button>
        <button type="button" className="btn btn--icon" onClick={onFit} aria-label="0">
          ⤢
        </button>
      </div>
    </div>
  );
}
