/**
 * Chart — the spatial instrument.
 *
 * The map is the primary surface: a chart of the chunk lattice inspected through analytical
 * lenses. The chunk register and the elevation profile sit beneath it as supporting reads of
 * the same selection, so the area is one instrument rather than three views of one dataset.
 *
 * Chunk coordinates are chart-qualified lattice addresses. The map projects one chart at one
 * containment layer and draws everything beyond the received extent as unsurveyed, because a
 * seamless planetary surface is not something this protocol can honestly show (INV-036).
 */

import { useMemo, useState } from "react";

import {
  Division,
  Field,
  Fields,
  Panel,
  Tag,
  TraceChip,
  Unsurveyed,
} from "../components/primitives";
import {
  formatChunkAddress,
  formatCompact,
  formatInteger,
  formatMillimetresAsMetres,
  type ObserverLocale,
} from "../observer/format";
import { useCopy, useFeed, useSession } from "../observer/instance";
import {
  buildAtlas,
  buildGates,
  buildLadders,
  CHUNK_SIZE,
  decodeCellOrdinal,
} from "../observer/models";
import { SIGNAL_VARIABLE } from "../observer/models";
import { ChartMap, type HoverReading } from "../map/ChartMap";
import {
  AVAILABILITY_DETAIL,
  AVAILABILITY_TITLE,
  LENS_GROUPS,
  availabilityOf,
  type LensContext,
} from "../map/lens";
import { LENSES, LENS_BY_ID, lensLayers, resolveLenses } from "../map/lenses";
import { LensDock } from "../map/LensDock";
import { LensIcon } from "../map/LensIcon";
import { LensLegend } from "../map/LensLegend";
import { Transect } from "../viz/Transect";
import type { AreaProps } from "../workspace";

function useChartContext(traceFilter?: bigint): LensContext {
  const locale = useSession((state) => state.locale);
  const world = useSession((state) => state.world);
  const rasters = useSession((state) => state.rasters);
  const summary = useSession((state) => state.summary);
  const atlas = useMemo(() => buildAtlas(world), [world]);
  const ladders = useMemo(() => buildLadders(world?.materialSurfaceDeltas ?? []), [world]);
  const gates = useMemo(() => buildGates(world?.materialSurfaceGateDeltas ?? []), [world]);
  return useMemo(
    () => ({ atlas, ladders, gates, rasters, traceFilter, summary, locale }),
    [atlas, gates, ladders, locale, rasters, summary, traceFilter],
  );
}

export function ChartArea({ workspace, update }: AreaProps) {
  useFeed("world");
  const copy = useCopy();
  const context = useChartContext(workspace.traceFilter);
  const locale = context.locale;
  const [hover, setHover] = useState<HoverReading>();

  const primary = LENS_BY_ID.get(workspace.primaryLens);
  const overlays = useMemo(() => resolveLenses(workspace.overlayLenses), [workspace.overlayLenses]);
  const primaryLayers = useMemo(
    () => (primary === undefined ? {} : lensLayers(primary, context)),
    [primary, context],
  );
  const overlayLayers = useMemo(
    () => overlays.map((lens) => ({ lens, layers: lensLayers(lens, context) })),
    [overlays, context],
  );

  const toggleOverlay = (id: string) => {
    const current = workspace.overlayLenses;
    update({
      overlayLenses: current.includes(id)
        ? current.filter((entry) => entry !== id)
        : [...current, id],
    });
  };

  const chartCount = context.atlas.charts.length;
  const awaitingPrimary = primary?.availability === "awaiting";

  return (
    <>
      <div className="chart-stage">
      <div className="area-head">
        <div className="area-head__meta">
          <Tag tone="quiet">
            {copy.chart.chartsPlural} {chartCount} · {copy.chart.chunksPlural}{" "}
            {context.atlas.chunks.length}
          </Tag>
        </div>
      </div>

      <div className="chart-frame">
        <LensDock
          locale={locale}
          context={context}
          primaryId={workspace.primaryLens}
          overlayIds={workspace.overlayLenses}
          onPrimary={(id) => update({ primaryLens: id })}
          onToggleOverlay={toggleOverlay}
          onOpenCatalogue={() => update({ dockCollapsed: false, dockView: "catalogue" })}
          labels={{
            primary: copy.chart.lens,
            overlays: copy.chart.overlays,
            catalogue: copy.chart.catalogue,
            awaiting: copy.chart.awaitingChip,
          }}
        />
        {context.atlas.chunks.length === 0 ? (
          <div className="chart-frame__empty">
            <Unsurveyed title={copy.chart.noChart} centred>
              {copy.chart.noChartBody}
            </Unsurveyed>
          </div>
        ) : (
          <>
            <ChartMap
              context={context}
              primary={awaitingPrimary ? undefined : primary}
              primaryLayers={awaitingPrimary ? {} : primaryLayers}
              overlayLayers={overlayLayers}
              selection={{ chunkKey: workspace.selectedChunk, cell: workspace.selectedCell }}
              onSelect={(selection) =>
                update({ selectedChunk: selection.chunkKey, selectedCell: selection.cell })
              }
              onHover={setHover}
              labels={{
                unsurveyed: copy.chart.unsurveyed,
                chunkAggregate: copy.chart.chunkAggregate,
                scale: copy.chart.scaleUnit,
                north: copy.chart.north,
              }}
              ariaLabel={copy.chart.title}
            />

            <LensLegend
              locale={locale}
              context={context}
              primary={primary}
              primaryLayers={awaitingPrimary ? {} : primaryLayers}
              overlays={overlayLayers}
              labels={{
                legend: copy.chart.legend,
                noField: copy.chart.noField,
                overlays: copy.chart.overlays,
              }}
            />

            <div className="chart-frame__readout">
              {hover === undefined ? (
                <span className="muted">{copy.chart.readoutIdle}</span>
              ) : (
                <>
                  <span className="numeric">
                    {hover.chunkX}, {hover.chunkY}
                  </span>
                  {hover.cell !== undefined && (
                    <span className="numeric muted">
                      · {copy.chart.cell} {hover.cell.x}, {hover.cell.y}
                    </span>
                  )}
                  <span className="numeric">
                    {hover.chunk === undefined ? copy.chart.unsurveyedShort : (hover.value ?? "—")}
                  </span>
                </>
              )}
            </div>

            {awaitingPrimary && primary !== undefined && (
              <div className="chart-frame__awaiting">
                <Unsurveyed title={primary.title[locale]} centred>
                  {primary.caveat?.[locale] ?? AVAILABILITY_DETAIL.awaiting[locale]}
                </Unsurveyed>
              </div>
            )}
          </>
        )}
      </div>

      <p className="chart__caption">{copy.chart.mapHint}</p>
      </div>

      {context.atlas.chunks.length > 0 && (
        <>
          <Panel
            title={copy.chart.profile}
            eyebrow={copy.common.projection}
            lede={copy.chart.profileLede}
            flushBody
          >
            <Transect
              atlas={context.atlas}
              selectedKey={workspace.selectedChunk}
              onSelect={(chunk) => update({ selectedChunk: chunk.key, selectedCell: undefined })}
              labels={{
                relief: copy.chart.elevation,
                mana: copy.chart.mana,
                population: copy.chart.population,
                activity: copy.chart.events,
                datum: `0 ${copy.chart.metres}`,
              }}
              ariaLabel={copy.chart.profile}
            />
          </Panel>

          <Panel
            title={copy.chart.register}
            eyebrow={`${copy.chart.chartsPlural} ${chartCount} · ${copy.chart.chunksPlural} ${context.atlas.chunks.length}`}
            flushBody
          >
            <div className="table-frame" style={{ ["--table-height" as string]: "20rem" }}>
              <table className="data">
                <thead>
                  <tr>
                    <th>{copy.chart.chunk}</th>
                    <th className="num">{copy.chart.elevationRange}</th>
                    <th className="num">{copy.chart.roughness}</th>
                    <th className="num">{copy.chart.mana}</th>
                    <th className="num">{copy.chart.resolution}</th>
                    <th className="num">{copy.chart.population}</th>
                    <th className="num">{copy.chart.events}</th>
                    <th className="num">{copy.chart.transitions}</th>
                    <th className="num">{copy.chart.latestTrace}</th>
                  </tr>
                </thead>
                <tbody>
                  {context.atlas.chunks.map((chunk) => (
                    <tr
                      key={chunk.key}
                      data-selected={workspace.selectedChunk === chunk.key}
                      onClick={() => update({ selectedChunk: chunk.key, selectedCell: undefined })}
                      style={{ cursor: "pointer" }}
                    >
                      <td className="numeric emphasis">{formatChunkAddress(chunk)}</td>
                      <td className="num">
                        {formatMillimetresAsMetres(chunk.minimumElevationMm, 1)} …{" "}
                        {formatMillimetresAsMetres(chunk.maximumElevationMm, 1)} {copy.chart.metres}
                      </td>
                      <td className="num">
                        {chunk.meanRoughnessMm} {copy.chart.millimetres}
                      </td>
                      <td className="num" style={{ color: "var(--sig-mana)" }}>
                        {formatCompact(chunk.manaTotal, locale)}
                      </td>
                      <td className="num" style={{ color: "var(--sig-resolution)" }}>
                        L{chunk.resolutionLevel} · {formatInteger(chunk.resolutionRelevance, locale)}
                      </td>
                      <td className="num" style={{ color: "var(--sig-life)" }}>
                        {formatInteger(chunk.populationTotal, locale)}
                      </td>
                      <td className="num">{formatInteger(chunk.causalEventCount, locale)}</td>
                      <td className="num">{chunk.transitions === 0 ? "—" : chunk.transitions}</td>
                      <td className="num muted">#{chunk.latestTraceId.toString()}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Panel>
        </>
      )}
    </>
  );
}

/* ------------------------------------------------------------------ dock -- */

export function ChartDock({ workspace, update }: AreaProps) {
  const copy = useCopy();
  const context = useChartContext(workspace.traceFilter);
  const locale = context.locale;
  const showCatalogue = workspace.dockView === "catalogue";

  const chunk = context.atlas.chunks.find((entry) => entry.key === workspace.selectedChunk);
  const chunkLadders = context.ladders.filter(
    (ladder) =>
      chunk !== undefined &&
      ladder.chartId === chunk.chartId &&
      ladder.chunkX === chunk.chunkX &&
      ladder.chunkY === chunk.chunkY &&
      ladder.chunkZ === chunk.chunkZ,
  );

  return (
    <>
      <div className="segmented segmented--text" role="group" aria-label={copy.chart.catalogue}>
        <button
          type="button"
          className="segmented__option"
          aria-pressed={!showCatalogue}
          onClick={() => update({ dockView: "selection" })}
        >
          {copy.chart.selection}
        </button>
        <button
          type="button"
          className="segmented__option"
          aria-pressed={showCatalogue}
          onClick={() => update({ dockView: "catalogue" })}
        >
          {copy.chart.catalogue}
        </button>
      </div>

      {showCatalogue ? (
        <LensCatalogue
          locale={locale}
          context={context}
          primaryId={workspace.primaryLens}
          overlayIds={workspace.overlayLenses}
          onPrimary={(id) => update({ primaryLens: id })}
          onToggleOverlay={(id) =>
            update({
              overlayLenses: workspace.overlayLenses.includes(id)
                ? workspace.overlayLenses.filter((entry) => entry !== id)
                : [...workspace.overlayLenses, id],
            })
          }
        />
      ) : chunk === undefined ? (
        <Unsurveyed title={copy.chart.selectPrompt}>{copy.chart.selectPromptBody}</Unsurveyed>
      ) : (
        <>
          <Panel variant="flush" eyebrow={copy.chart.chunk} title={formatChunkAddress(chunk)}>
            <Fields>
              <Field label={copy.chart.chart}>#{chunk.chartId.toString()}</Field>
              <Field label={copy.chart.elevationRange}>
                {formatMillimetresAsMetres(chunk.minimumElevationMm)} …{" "}
                {formatMillimetresAsMetres(chunk.maximumElevationMm)} {copy.chart.metres}
              </Field>
              <Field label={copy.chart.roughness}>
                {chunk.meanRoughnessMm} {copy.chart.millimetres}
              </Field>
              <Field label={copy.chart.mana}>{formatInteger(chunk.manaTotal, locale)}</Field>
              <Field label={copy.chart.resolution}>
                L{chunk.resolutionLevel} · {formatInteger(chunk.resolutionRelevance, locale)}
              </Field>
              <Field label={copy.chart.population}>
                {formatInteger(chunk.populationTotal, locale)}
              </Field>
              <Field label={copy.chart.events}>
                {formatInteger(chunk.causalEventCount, locale)}
              </Field>
              <Field label={copy.chart.latestTrace}>
                <TraceChip id={chunk.latestTraceId} />
              </Field>
            </Fields>
          </Panel>

          {workspace.selectedCell !== undefined && (
            <Panel variant="flush" eyebrow={copy.chart.cell} title={`${workspace.selectedCell.x}, ${workspace.selectedCell.y}`}>
              <p className="lede">{copy.chart.cellLede}</p>
            </Panel>
          )}

          <Panel
            variant="flush"
            eyebrow={copy.common.bounded}
            title={copy.chart.surfaceCell}
            lede={copy.chart.surfaceCellLede}
          >
            {chunkLadders.length === 0 ? (
              <p className="lede">{copy.flux.noLadderBody}</p>
            ) : (
              <Fields>
                {chunkLadders.map((ladder) => {
                  const cell = decodeCellOrdinal(ladder.cellOrdinal);
                  const last = ladder.steps[ladder.steps.length - 1];
                  return (
                    <Field
                      key={ladder.key}
                      label={`#${ladder.cellOrdinal} → (${cell.x}, ${cell.y}, ${cell.z}) / ${CHUNK_SIZE}³`}
                    >
                      {last === undefined
                        ? copy.common.none
                        : `${copy.flux.condition} ${last.afterCondition} · ${ladder.steps.length}`}
                    </Field>
                  );
                })}
              </Fields>
            )}
          </Panel>
        </>
      )}
    </>
  );
}

function LensCatalogue({
  locale,
  context,
  primaryId,
  overlayIds,
  onPrimary,
  onToggleOverlay,
}: {
  locale: ObserverLocale;
  context: LensContext;
  primaryId: string;
  overlayIds: readonly string[];
  onPrimary(id: string): void;
  onToggleOverlay(id: string): void;
}) {
  return (
    <>
      {LENS_GROUPS.map((group) => {
        const lenses = LENSES.filter((lens) => lens.group === group.id);
        if (lenses.length === 0) return null;
        return (
          <section key={group.id}>
            <Division>{group.title[locale]}</Division>
            {lenses.map((lens) => availabilityEntry(lens, context)).map(({ lens, state }) => (
              <article key={lens.id} className="lens-entry" data-availability={state}>
                <button
                  type="button"
                  className="lens-entry__head"
                  aria-pressed={lens.id === primaryId || overlayIds.includes(lens.id)}
                  style={{ ["--signal" as string]: SIGNAL_VARIABLE[lens.signal] }}
                  onClick={() => {
                    if (lens.roles.includes("primary")) onPrimary(lens.id);
                    else onToggleOverlay(lens.id);
                  }}
                >
                  <span className="lens-entry__mark" data-availability={state}>
                    <LensIcon lensId={lens.id} size={15} />
                  </span>
                  <span className="lens-entry__name">{lens.title[locale]}</span>
                  <span className="lens-availability" data-availability={state}>
                    {AVAILABILITY_TITLE[state][locale]}
                  </span>
                </button>
                <p className="lens-entry__detail">{lens.detail[locale]}</p>
                {lens.caveat !== undefined && (
                  <p className="lens-entry__caveat">{lens.caveat[locale]}</p>
                )}
              </article>
            ))}
          </section>
        );
      })}
    </>
  );
}

/** A lens with the availability it is actually in, given what has arrived. */
function availabilityEntry(lens: (typeof LENSES)[number], context: LensContext) {
  return { lens, state: availabilityOf(lens, context) };
}
