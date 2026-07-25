/**
 * Survey — the spatial workspace.
 *
 * Chunk coordinates are chart-qualified lattice addresses. The profile places them in
 * coordinate order and says so; it never presents them as a seamless global map, and it
 * never draws terrain it was not given (INV-036, INV-037).
 */

import { useMemo } from "react";

import {
  Derived,
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
} from "../observer/format";
import { useCopy, useFeed, useSession } from "../observer/instance";
import { buildAtlas, buildLadders, decodeCellOrdinal, CHUNK_SIZE } from "../observer/models";
import { Transect } from "../viz/Transect";
import type { AreaProps } from "../workspace";

export function SurveyArea({ workspace, update }: AreaProps) {
  useFeed("world");
  const copy = useCopy();
  const locale = useSession((state) => state.locale);
  const world = useSession((state) => state.world);
  const worldTicks = useSession((state) => state.worldTicks);

  const atlas = useMemo(() => buildAtlas(world), [world]);

  if (atlas.chunks.length === 0) {
    return (
      <>
        <SurveyHead copy={copy} ticks={worldTicks} />
        <Unsurveyed title={copy.survey.noWorld}>{copy.survey.noWorldBody}</Unsurveyed>
      </>
    );
  }

  return (
    <>
      <SurveyHead copy={copy} ticks={worldTicks} />

      <Panel
        title={copy.survey.transect}
        eyebrow={copy.survey.eyebrow}
        lede={copy.survey.transectLede}
        tools={<Tag tone="quiet">{copy.common.projection}</Tag>}
        flushBody
        foot={
          <>
            <span className="transect__scale">
              <span className="transect__ramp" aria-hidden="true" />
              {copy.survey.mana}
            </span>
            <span className="transect__scale">
              <span
                className="glyph"
                style={{ background: "var(--sig-life)" }}
                aria-hidden="true"
              />
              {copy.survey.population}
            </span>
            <span className="transect__scale">
              <span
                className="glyph"
                style={{ borderLeft: "1px solid var(--sig-physical)" }}
                aria-hidden="true"
              />
              {copy.survey.events}
            </span>
            <span className="transect__scale">
              <span className="glyph glyph--mana" aria-hidden="true" />
              {copy.survey.transitions}
            </span>
            <span className="transect__scale">
              <span
                className="glyph"
                style={{ background: "var(--sig-resolution)" }}
                aria-hidden="true"
              />
              {copy.survey.resolution}
            </span>
          </>
        }
      >
        <Transect
          atlas={atlas}
          selectedKey={workspace.selectedChunk}
          onSelect={(chunk) => update({ selectedChunk: chunk.key })}
          labels={{
            relief: copy.survey.elevation,
            mana: copy.survey.mana,
            population: copy.survey.population,
            activity: copy.survey.events,
            datum: `0 ${copy.survey.metres}`,
          }}
          ariaLabel={copy.survey.transect}
        />
      </Panel>

      <Panel
        title={copy.survey.register}
        eyebrow={`${copy.survey.chartsPlural} ${atlas.charts.length} · ${copy.survey.chunksPlural} ${atlas.chunks.length}`}
        flushBody
      >
        <div className="table-frame" style={{ ["--table-height" as string]: "22rem" }}>
          <table className="data">
            <thead>
              <tr>
                <th>{copy.survey.chunk}</th>
                <th className="num">{copy.survey.elevationRange}</th>
                <th className="num">{copy.survey.roughness}</th>
                <th className="num">{copy.survey.mana}</th>
                <th className="num">{copy.survey.resolution}</th>
                <th className="num">{copy.survey.population}</th>
                <th className="num">{copy.survey.events}</th>
                <th className="num">{copy.survey.transitions}</th>
                <th className="num">{copy.survey.latestTrace}</th>
              </tr>
            </thead>
            <tbody>
              {atlas.chunks.map((chunk) => (
                <tr
                  key={chunk.key}
                  data-selected={workspace.selectedChunk === chunk.key}
                  onClick={() => update({ selectedChunk: chunk.key })}
                  style={{ cursor: "pointer" }}
                >
                  <td className="numeric emphasis">{formatChunkAddress(chunk)}</td>
                  <td className="num">
                    {formatMillimetresAsMetres(chunk.minimumElevationMm, 1)} …{" "}
                    {formatMillimetresAsMetres(chunk.maximumElevationMm, 1)} {copy.survey.metres}
                  </td>
                  <td className="num">
                    {chunk.meanRoughnessMm} {copy.survey.millimetres}
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
  );
}

function SurveyHead({ copy, ticks }: { copy: ReturnType<typeof useCopy>; ticks?: bigint }) {
  return (
    <div className="area-head">
      <div className="area-head__titles">
        <span className="eyebrow">{copy.survey.eyebrow}</span>
        <h1 className="display">{copy.survey.title}</h1>
        <p className="lede">{copy.survey.lede}</p>
      </div>
      <div className="area-head__meta">
        <Derived>
          {copy.transport.ticks} {ticks === undefined ? "—" : ticks.toString()}
        </Derived>
      </div>
    </div>
  );
}

export function SurveyDock({ workspace }: AreaProps) {
  const copy = useCopy();
  const locale = useSession((state) => state.locale);
  const world = useSession((state) => state.world);
  const atlas = useMemo(() => buildAtlas(world), [world]);
  const ladders = useMemo(
    () => buildLadders(world?.materialSurfaceDeltas ?? []),
    [world],
  );

  const chunk = atlas.chunks.find((entry) => entry.key === workspace.selectedChunk);
  if (chunk === undefined) {
    return <Unsurveyed title={copy.survey.selectPrompt} />;
  }

  const chunkLadders = ladders.filter(
    (ladder) =>
      ladder.chartId === chunk.chartId &&
      ladder.chunkX === chunk.chunkX &&
      ladder.chunkY === chunk.chunkY &&
      ladder.chunkZ === chunk.chunkZ,
  );
  const relief = chunk.maximumElevationMm - chunk.minimumElevationMm;

  return (
    <>
      <Panel variant="flush" eyebrow={copy.survey.chunk} title={formatChunkAddress(chunk)}>
        <Fields>
          <Field label={copy.survey.chart}>#{chunk.chartId.toString()}</Field>
          <Field label={copy.survey.elevationRange}>
            {formatMillimetresAsMetres(chunk.minimumElevationMm)} …{" "}
            {formatMillimetresAsMetres(chunk.maximumElevationMm)} {copy.survey.metres}
          </Field>
          <Field label={copy.survey.elevation}>
            Δ {formatMillimetresAsMetres(relief)} {copy.survey.metres}
          </Field>
          <Field label={copy.survey.roughness}>
            {chunk.meanRoughnessMm} {copy.survey.millimetres}
          </Field>
          <Field label={copy.survey.mana}>{formatInteger(chunk.manaTotal, locale)}</Field>
          <Field label={copy.survey.resolution}>
            L{chunk.resolutionLevel} · {formatInteger(chunk.resolutionRelevance, locale)}
          </Field>
          <Field label={copy.survey.population}>
            {formatInteger(chunk.populationTotal, locale)}
          </Field>
          <Field label={copy.survey.events}>
            {formatInteger(chunk.causalEventCount, locale)}
          </Field>
          <Field label={copy.survey.latestTrace}>
            <TraceChip id={chunk.latestTraceId} />
          </Field>
        </Fields>
      </Panel>

      <Panel
        variant="flush"
        eyebrow={copy.common.bounded}
        title={copy.survey.surfaceCell}
        lede={copy.survey.surfaceCellLede}
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
                    : `${copy.flux.condition} ${last.afterCondition} · ${ladder.steps.length} ${copy.survey.transitions.toLowerCase()}`}
                </Field>
              );
            })}
          </Fields>
        )}
      </Panel>
    </>
  );
}
