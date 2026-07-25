/**
 * The surface condition ladder.
 *
 * Every tracked material surface is a step function over run ticks: each committed
 * transition raises the condition and carries the trace that caused it. All lines measure
 * the same quantity, so they share one hue; identity comes from the direct label at the end
 * of each line and from selection, not from a colour per surface.
 *
 * Markers are provenance, not magnitude: a ring is a contact trace, a diamond is a mana
 * physical effect, a square is a local mana coupling event.
 */

import { useCallback, useState } from "react";

import type { SurfaceLadder, SurfaceStep } from "../observer/models";
import { ladderExtent } from "../observer/models";
import { CanvasSurface, MONO_FONT, readPalette, type Frame } from "./canvas";
import { linearScale, niceTicks, axisLabel } from "./scale";
import { withAlpha } from "./ChartRecorder";

const PADDING = { top: 14, right: 78, bottom: 22, left: 44 };

interface LadderProbe {
  ladderKey: string;
  step: SurfaceStep;
  x: number;
  y: number;
  side: "left" | "right";
}

interface ConditionLadderProps {
  ladders: SurfaceLadder[];
  selectedKey?: string;
  onSelect?(key: string): void;
  height?: number;
  ariaLabel: string;
  tickLabel: string;
  conditionLabel: string;
  labelFor(ladder: SurfaceLadder): string;
  probeLabels: { condition: string; mana: string; localMana: string; contact: string };
}

export function ConditionLadder({
  ladders,
  selectedKey,
  onSelect,
  height = 226,
  ariaLabel,
  tickLabel,
  conditionLabel,
  labelFor,
  probeLabels,
}: ConditionLadderProps) {
  const [probe, setProbe] = useState<LadderProbe>();

  const draw = useCallback(
    (context: CanvasRenderingContext2D, frame: Frame) => {
      if (ladders.length === 0) return;
      const palette = readPalette();
      const extent = ladderExtent(ladders);
      const plotWidth = frame.width - PADDING.left - PADDING.right;
      const plotHeight = frame.height - PADDING.top - PADDING.bottom;
      if (plotWidth <= 0 || plotHeight <= 0) return;

      const x = linearScale(
        [extent.tickMin, extent.tickMax],
        [PADDING.left, PADDING.left + plotWidth],
      );
      const y = linearScale(
        [extent.conditionMin, extent.conditionMax],
        [PADDING.top + plotHeight, PADDING.top],
      );

      context.font = MONO_FONT;
      context.textBaseline = "middle";
      context.textAlign = "right";
      for (const tick of niceTicks(extent.conditionMin, extent.conditionMax, 4)) {
        const py = Math.round(y(tick)) + 0.5;
        context.strokeStyle = palette.ruleGhost!;
        context.lineWidth = 1;
        context.setLineDash([1, 3]);
        context.beginPath();
        context.moveTo(PADDING.left, py);
        context.lineTo(PADDING.left + plotWidth, py);
        context.stroke();
        context.setLineDash([]);
        context.fillStyle = palette.inkGhost!;
        context.fillText(axisLabel(tick), PADDING.left - 8, py);
      }

      context.textAlign = "center";
      context.textBaseline = "top";
      for (const tick of niceTicks(extent.tickMin, extent.tickMax, 5)) {
        const px = Math.round(x(tick)) + 0.5;
        context.strokeStyle = palette.ruleGhost!;
        context.setLineDash([1, 3]);
        context.beginPath();
        context.moveTo(px, PADDING.top);
        context.lineTo(px, PADDING.top + plotHeight);
        context.stroke();
        context.setLineDash([]);
        context.fillStyle = palette.inkGhost!;
        context.fillText(axisLabel(tick), px, PADDING.top + plotHeight + 6);
      }

      context.strokeStyle = palette.rule!;
      context.beginPath();
      context.moveTo(PADDING.left + 0.5, PADDING.top);
      context.lineTo(PADDING.left + 0.5, PADDING.top + plotHeight + 0.5);
      context.lineTo(PADDING.left + plotWidth, PADDING.top + plotHeight + 0.5);
      context.stroke();

      for (const ladder of ladders) {
        const selected = ladder.key === selectedKey;
        const dimmed = selectedKey !== undefined && !selected;
        const color = selected ? palette.mark! : palette.physical!;
        const alpha = dimmed ? 0.32 : 1;

        // Step-after: a condition holds until the next committed transition.
        context.strokeStyle = withAlpha(color, alpha);
        context.lineWidth = selected ? 2 : 1.5;
        context.lineJoin = "round";
        context.beginPath();
        ladder.steps.forEach((step, index) => {
          const px = x(step.tick);
          const pyBefore = y(step.beforeCondition);
          const pyAfter = y(step.afterCondition);
          if (index === 0) context.moveTo(px, pyBefore);
          else context.lineTo(px, pyBefore);
          context.lineTo(px, pyAfter);
        });
        const last = ladder.steps[ladder.steps.length - 1];
        if (last !== undefined) {
          context.lineTo(x(extent.tickMax), y(last.afterCondition));
        }
        context.stroke();

        for (const step of ladder.steps) {
          const px = x(step.tick);
          const py = y(step.afterCondition);
          if (step.contactTraceId !== undefined) {
            context.strokeStyle = withAlpha(palette.ruleBright!, alpha);
            context.lineWidth = 1;
            context.beginPath();
            context.arc(px, py, 2.4, 0, Math.PI * 2);
            context.stroke();
          }
          if (step.manaEffectTraceId !== undefined) {
            context.fillStyle = withAlpha(palette.mana!, alpha);
            context.strokeStyle = palette.paper!;
            context.lineWidth = 1.5;
            context.beginPath();
            context.moveTo(px, py - 4.5);
            context.lineTo(px + 4.5, py);
            context.lineTo(px, py + 4.5);
            context.lineTo(px - 4.5, py);
            context.closePath();
            context.fill();
            context.stroke();
          }
          if (step.localManaAfter !== undefined) {
            context.strokeStyle = withAlpha(palette.resolution!, alpha);
            context.fillStyle = withAlpha(palette.resolution!, alpha * 0.3);
            context.lineWidth = 1;
            context.beginPath();
            context.rect(px - 3.5, py - 3.5, 7, 7);
            context.fill();
            context.stroke();
          }
        }

        // Direct label at the end of the line; identity never depends on colour alone.
        if (last !== undefined) {
          context.font = MONO_FONT;
          context.textAlign = "left";
          context.textBaseline = "middle";
          context.fillStyle = selected ? palette.mark! : withAlpha(palette.inkFaint!, alpha);
          context.fillText(labelFor(ladder), PADDING.left + plotWidth + 8, y(last.afterCondition));
        }
      }

      // Axis captions.
      context.font = MONO_FONT;
      context.textAlign = "left";
      context.textBaseline = "top";
      context.fillStyle = palette.inkGhost!;
      context.fillText(conditionLabel, PADDING.left - 38, 2);
      context.textAlign = "right";
      context.fillText(tickLabel, PADDING.left + plotWidth, PADDING.top + plotHeight + 6);
    },
    [conditionLabel, labelFor, ladders, selectedKey, tickLabel],
  );

  const locate = useCallback(
    (point: { x: number; y: number; frame: Frame } | undefined): LadderProbe | undefined => {
      if (point === undefined || ladders.length === 0) return undefined;
      const extent = ladderExtent(ladders);
      const plotWidth = point.frame.width - PADDING.left - PADDING.right;
      const plotHeight = point.frame.height - PADDING.top - PADDING.bottom;
      if (plotWidth <= 0 || plotHeight <= 0) return undefined;
      const x = linearScale(
        [extent.tickMin, extent.tickMax],
        [PADDING.left, PADDING.left + plotWidth],
      );
      const y = linearScale(
        [extent.conditionMin, extent.conditionMax],
        [PADDING.top + plotHeight, PADDING.top],
      );
      let best: LadderProbe | undefined;
      let bestDistance = 18;
      for (const ladder of ladders) {
        for (const step of ladder.steps) {
          const px = x(step.tick);
          const py = y(step.afterCondition);
          const distance = Math.hypot(px - point.x, py - point.y);
          if (distance < bestDistance) {
            bestDistance = distance;
            best = {
              ladderKey: ladder.key,
              step,
              x: px,
              y: py,
              side: px > point.frame.width * 0.55 ? "right" : "left",
            };
          }
        }
      }
      return best;
    },
    [ladders],
  );

  return (
    <div className="chart">
      <CanvasSurface
        height={height}
        draw={draw}
        label={ariaLabel}
        onProbe={(point) => setProbe(locate(point))}
        onActivate={(point) => {
          const found = locate(point);
          if (found !== undefined && onSelect !== undefined) {
            onSelect(found.ladderKey === selectedKey ? "" : found.ladderKey);
          }
        }}
      >
        {probe !== undefined && (
          <div
            className="chart__probe"
            style={
              probe.side === "left"
                ? { left: probe.x + 14, top: Math.max(6, probe.y - 30) }
                : { right: 14, top: Math.max(6, probe.y - 30) }
            }
          >
            <div className="chart__probe-head">
              <span>{tickLabel}</span>
              <b className="numeric">{probe.step.tick}</b>
            </div>
            <div className="chart__probe-row" style={{ ["--signal" as string]: "var(--sig-physical)" }}>
              <span className="chart__probe-mark" />
              <span>{probeLabels.condition}</span>
              <b>
                {probe.step.beforeCondition} → {probe.step.afterCondition}
              </b>
            </div>
            <div className="chart__probe-row" style={{ ["--signal" as string]: "var(--sig-mana)" }}>
              <span className="chart__probe-mark" />
              <span>{probeLabels.mana}</span>
              <b>{probe.step.manaTotal}</b>
            </div>
            {probe.step.localManaAfter !== undefined && (
              <div
                className="chart__probe-row"
                style={{ ["--signal" as string]: "var(--sig-resolution)" }}
              >
                <span className="chart__probe-mark" />
                <span>{probeLabels.localMana}</span>
                <b>
                  {probe.step.localManaBefore} → {probe.step.localManaAfter}
                </b>
              </div>
            )}
            {probe.step.contactTraceId !== undefined && (
              <div className="chart__probe-row" style={{ ["--signal" as string]: "var(--line-bright)" }}>
                <span className="chart__probe-mark" />
                <span>{probeLabels.contact}</span>
                <b>#{probe.step.contactTraceId.toString()}</b>
              </div>
            )}
          </div>
        )}
      </CanvasSurface>
    </div>
  );
}
