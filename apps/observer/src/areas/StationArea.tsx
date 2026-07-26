/**
 * Observatory — the primary surface.
 *
 * The run's identity, the instrument cluster, and the three quantities that actually move in
 * a live session: the mana field, causal accretion, and actor action admission. Everything
 * here is the objective observer projection; none of it is agent knowledge.
 */

import { useMemo } from "react";

import {
  DigestPlate,
  Field,
  Fields,
  Meter,
  Panel,
  RateTable,
  Readout,
  Tag,
} from "../components/primitives";
import { capabilityCounts } from "../observer/capability";
import {
  formatCompact,
  formatInteger,
  formatPercent,
  formatTraceId,
} from "../observer/format";
import { useCopy, useFeed, useSession } from "../observer/instance";
import {
  activityRates,
  admissionRatio,
  buildAtlas,
  buildLedger,
  levelSeries,
  type LedgerEntry,
  type SignalId,
} from "../observer/models";
import { ChartRecorder } from "../viz/ChartRecorder";
import { Sparkline } from "../viz/Sparkline";
import { Transect } from "../viz/Transect";
import type { AreaProps } from "../workspace";

function contactTrace(entry: LedgerEntry): bigint | undefined {
  return entry.traces.find((trace) => trace.roles.includes("contact"))?.id;
}

export function StationArea({ update, goTo }: AreaProps) {
  useFeed("world");
  const copy = useCopy();
  const locale = useSession((state) => state.locale);
  const summary = useSession((state) => state.summary);
  const history = useSession((state) => state.history);
  const world = useSession((state) => state.world);
  const negotiation = useSession((state) => state.negotiation);
  const seed = useSession((state) => state.seed);

  const atlas = useMemo(() => buildAtlas(world), [world]);
  const ledger = useMemo(
    () => buildLedger(world?.materialSurfaceDeltas ?? []).slice(0, 6),
    [world],
  );
  const rates = useMemo(() => activityRates(history), [history]);
  const admission = admissionRatio(summary);
  const coverage = useMemo(() => capabilityCounts(), []);

  const manaSeries = useMemo(
    () => [
      levelSeries(history, "manaTotal", "mana", (item) => item.manaTotal),
      { ...levelSeries(history, "manaPeak", "mana", (item) => item.manaMaximum), dashed: true },
    ],
    [history],
  );
  const accretionSeries = useMemo(
    () => [levelSeries(history, "traces", "trace", (item) => item.causalTraceCount)],
    [history],
  );
  const admissionSeries = useMemo(
    () => [
      levelSeries(history, "committed", "life", (item) => item.actorActionsCommitted),
      levelSeries(history, "rejected", "refused", (item) => item.actorActionsRejected),
    ],
    [history],
  );

  const sparkFor = (signal: SignalId, read: (item: (typeof history)[number]) => bigint) =>
    levelSeries(history, "spark", signal, read).points;

  return (
    <>
      <div className="area-head">
        <div className="area-head__meta">
          <Readout
            label={copy.station.traces}
            value={summary === undefined ? "—" : formatCompact(summary.causalTraceCount, locale)}
            note={
              rates.get("traces")?.available === true
                ? `${rates.get("traces")!.perTick.toFixed(1)} ${copy.flux.perTick}`
                : undefined
            }
            signal="trace"
            size="hero"
            align="end"
          />
        </div>
      </div>

      <Panel variant="flush" flushBody>
        <div className="cluster">
          <Readout
            label={copy.station.manaTotal}
            value={summary === undefined ? "—" : formatCompact(summary.manaTotal, locale)}
            signal="mana"
          >
            <Sparkline
              points={sparkFor("mana", (item) => item.manaTotal)}
              signal="mana"
              label={copy.station.manaTotal}
            />
          </Readout>
          <Readout
            label={copy.station.population}
            value={summary === undefined ? "—" : formatInteger(summary.populationTotal, locale)}
            note={
              summary === undefined
                ? undefined
                : `+${summary.populationBirths} · −${summary.populationDeaths}`
            }
            signal="life"
          />
          <Readout
            label={copy.station.actors}
            value={summary === undefined ? "—" : formatInteger(summary.actorCount, locale)}
            signal="life"
          />
          <Readout
            label={copy.station.activeChunks}
            value={summary === undefined ? "—" : formatInteger(summary.activeChunkCount, locale)}
            signal="physical"
          />
          <Readout
            label={copy.station.physicalEvents}
            value={summary === undefined ? "—" : formatInteger(summary.physicalEvents, locale)}
            signal="physical"
          >
            <Sparkline
              points={sparkFor("physical", (item) => item.physicalEvents)}
              signal="physical"
              label={copy.station.physicalEvents}
              filled={false}
            />
          </Readout>
          <Readout
            label={copy.station.resolutionLevel}
            value={summary === undefined ? "—" : formatInteger(summary.resolutionLevel, locale)}
            note={
              summary === undefined
                ? undefined
                : `${copy.station.resolutionRelevance} ${formatInteger(summary.resolutionRelevance, locale)}`
            }
            signal="resolution"
          />
        </div>
      </Panel>

      <div className="grid grid--halves">
        <Panel title={copy.station.field} eyebrow={copy.common.projection} lede={copy.station.fieldLede}>
          <ChartRecorder
            series={manaSeries}
            label={copy.station.field}
            tickLabel={copy.transport.ticks}
            legendLabels={{ manaTotal: copy.station.manaTotal, manaPeak: copy.station.manaPeak }}
            emptyLabel={copy.chart.noWorld}
            fillFirst
          />
        </Panel>

        <Panel
          title={copy.station.accretion}
          eyebrow={copy.common.projection}
          lede={copy.station.accretionLede}
          tools={<Tag tone="quiet">{copy.common.derived}</Tag>}
        >
          <ChartRecorder
            series={accretionSeries}
            label={copy.station.accretion}
            tickLabel={copy.transport.ticks}
            legendLabels={{ traces: copy.station.traces }}
            emptyLabel={copy.chart.noWorld}
            fillFirst
          />
          <div className="cluster" style={{ marginTop: "var(--s2)" }}>
            <Readout
              label={copy.station.manaCellChanges}
              value={summary === undefined ? "—" : formatCompact(summary.manaCellChanges, locale)}
              signal="mana"
              size="compact"
            />
            <Readout
              label={copy.station.manaEffects}
              value={summary === undefined ? "—" : formatInteger(summary.manaPhysicalEffects, locale)}
              signal="mana"
              size="compact"
            />
            <Readout
              label={copy.station.resolutionTransitions}
              value={
                summary === undefined ? "—" : formatInteger(summary.resolutionTransitions, locale)
              }
              signal="resolution"
              size="compact"
            />
          </div>
        </Panel>
      </div>

      <div className="grid grid--wide-left">
        <Panel
          title={copy.station.chartStrip}
          eyebrow={copy.chart.eyebrow}
          lede={copy.station.chartStripLede}
          tools={
            <button type="button" className="btn" onClick={() => goTo("chart")}>
              {copy.areas.chart.name} →
            </button>
          }
          flushBody
        >
          {atlas.chunks.length === 0 ? (
            <div style={{ padding: "var(--s3)" }}>
              <p className="lede">{copy.chart.noWorld}</p>
            </div>
          ) : (
            <Transect
              atlas={atlas}
              onSelect={(chunk) => {
                update({ selectedChunk: chunk.key });
                goTo("chart");
              }}
              labels={{
                relief: copy.chart.elevation,
                mana: copy.chart.mana,
                population: copy.chart.population,
                activity: copy.chart.events,
                datum: "0 m",
              }}
              ariaLabel={copy.station.chartStrip}
            />
          )}
        </Panel>

        <Panel title={copy.station.admission} eyebrow={copy.common.projection} lede={copy.station.admissionLede}>
          <Meter
            fraction={admission ?? 0}
            signal="life"
            ticks
            left={`${copy.station.committed} ${summary === undefined ? "—" : formatInteger(summary.actorActionsCommitted, locale)}`}
            right={`${copy.station.rejected} ${summary === undefined ? "—" : formatInteger(summary.actorActionsRejected, locale)}`}
          />
          <p className="chart__caption" style={{ marginTop: "var(--s2)" }}>
            {admission === undefined ? copy.common.none : formatPercent(admission, 1)}
          </p>
          <ChartRecorder
            series={admissionSeries}
            height={120}
            label={copy.station.admission}
            tickLabel={copy.transport.ticks}
            legendLabels={{
              committed: copy.station.committed,
              rejected: copy.station.rejected,
            }}
            emptyLabel={copy.chart.noWorld}
          />
        </Panel>
      </div>

      <div className="grid grid--wide-right">
        <Panel
          title={copy.station.coverage}
          eyebrow={copy.instrument.eyebrow}
          lede={copy.station.coverageLede}
          tools={
            <button type="button" className="btn" onClick={() => goTo("instrument")}>
              {copy.station.openRegister} →
            </button>
          }
        >
          <Fields>
            <Field label={<Tag tone="live">{copy.instrument.stateLive}</Tag>}>{coverage.live}</Field>
            <Field label={<Tag tone="partial">{copy.instrument.stateBounded}</Tag>}>
              {coverage.bounded}
            </Field>
            <Field label={<Tag tone="unknown">{copy.instrument.stateAbsentProjection}</Tag>}>
              {coverage["absent-projection"]}
            </Field>
            <Field label={<Tag tone="unknown">{copy.instrument.stateAbsentDomain}</Tag>}>
              {coverage["absent-domain"]}
            </Field>
          </Fields>
        </Panel>

        <Panel
          title={copy.station.activity}
          eyebrow={copy.common.bounded}
          lede={copy.station.activityLede}
          flushBody
          tools={
            <button type="button" className="btn" onClick={() => goTo("flux")}>
              {copy.areas.flux.name} →
            </button>
          }
        >
          {ledger.length === 0 ? (
            <div style={{ padding: "var(--s3)" }}>
              <p className="lede">{copy.flux.noLadderBody}</p>
            </div>
          ) : (
            <div className="table-frame" style={{ ["--table-height" as string]: "13rem" }}>
              <table className="data">
                <thead>
                  <tr>
                    <th className="num">{copy.flux.tick}</th>
                    <th>{copy.chart.chunk}</th>
                    <th className="num">{copy.flux.condition}</th>
                    <th className="num">{copy.chart.mana}</th>
                    <th>{copy.flux.contact}</th>
                  </tr>
                </thead>
                <tbody>
                  {ledger.map((entry) => (
                    <tr key={entry.id}>
                      <td className="num emphasis">{entry.tick}</td>
                      <td className="numeric">
                        {entry.chunkX}, {entry.chunkY}, {entry.chunkZ}
                      </td>
                      <td className="num">
                        {entry.beforeCondition} → {entry.afterCondition}
                      </td>
                      <td className="num">{formatCompact(entry.manaTotal, locale)}</td>
                      <td className="numeric">
                        {contactTrace(entry) === undefined
                          ? copy.common.none
                          : formatTraceId(contactTrace(entry)!)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </Panel>
      </div>

      <Panel title={copy.station.identity} eyebrow={copy.instrument.negotiation} lede={copy.station.identityLede}>
        <div className="grid grid--thirds">
          <Fields>
            <Field label={copy.transport.seed}>{seed}</Field>
            <Field label={copy.instrument.protocolVersion}>
              v{negotiation?.protocolVersion ?? "—"}
            </Field>
            <Field label={copy.transport.ticks}>
              {summary === undefined ? "—" : summary.simulationTicks.toString()}
            </Field>
            <Field label={copy.station.bytesPerChunk}>
              {summary === undefined ? "—" : summary.bytesPerChunk.toString()}
            </Field>
            <Field label={copy.station.latestTrace}>
              {summary === undefined ? "—" : formatTraceId(summary.latestTraceId)}
            </Field>
          </Fields>
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--s3)" }}>
            {summary !== undefined && (
              <>
                <DigestPlate
                  label={copy.meridian.physicalDigest}
                  bytes={summary.physicalDigest}
                  count={10}
                />
                <DigestPlate
                  label={copy.meridian.historyDigest}
                  bytes={summary.historyDigest}
                  count={10}
                />
              </>
            )}
            <p className="chart__caption">{copy.meridian.digestNote}</p>
          </div>
          <Fields>
            <Field label={copy.station.births}>
              {summary === undefined ? "—" : summary.populationBirths.toString()}
            </Field>
            <Field label={copy.station.deaths}>
              {summary === undefined ? "—" : summary.populationDeaths.toString()}
            </Field>
            <Field label={copy.station.movements}>
              {summary === undefined ? "—" : summary.populationMovements.toString()}
            </Field>
          </Fields>
        </div>
      </Panel>
    </>
  );
}

export function StationDock() {
  const copy = useCopy();
  const locale = useSession((state) => state.locale);
  const summary = useSession((state) => state.summary);
  const history = useSession((state) => state.history);
  const frames = useSession((state) => state.history.length);
  const rates = useMemo(() => activityRates(history), [history]);

  if (summary === undefined) {
    return <p className="lede">{copy.chart.noWorld}</p>;
  }

  const rows: { id: string; label: string; signal: SignalId }[] = [
    { id: "traces", label: copy.station.traces, signal: "trace" },
    { id: "physicalEvents", label: copy.station.physicalEvents, signal: "physical" },
    { id: "manaCellChanges", label: copy.station.manaCellChanges, signal: "mana" },
    { id: "manaPhysicalEffects", label: copy.station.manaEffects, signal: "mana" },
    { id: "resolutionTransitions", label: copy.station.resolutionTransitions, signal: "resolution" },
    { id: "actorActionsCommitted", label: copy.station.committed, signal: "life" },
    { id: "actorActionsRejected", label: copy.station.rejected, signal: "refused" },
    { id: "populationMovements", label: copy.station.movements, signal: "life" },
    { id: "populationBirths", label: copy.station.births, signal: "life" },
    { id: "populationDeaths", label: copy.station.deaths, signal: "refused" },
  ];

  return (
    <>
      <Panel variant="flush" title={copy.flux.rates} eyebrow={copy.common.derived} lede={copy.flux.ratesLede}>
        <RateTable
          columns={{
            quantity: copy.flux.quantity,
            rate: copy.flux.perTick,
            total: copy.flux.total,
          }}
          rows={rows.map((row) => {
            const rate = rates.get(row.id);
            return {
              id: row.id,
              label: row.label,
              signal: row.signal,
              rate: rate?.available === true ? rate.perTick.toFixed(2) : undefined,
              total: rate === undefined ? "—" : formatInteger(rate.total, locale),
            };
          })}
        />
      </Panel>

      <Panel variant="flush" title={copy.instrument.boundsTitle} eyebrow={copy.common.bounded}>
        <Fields>
          <Field label={copy.instrument.boundFrames}>{frames}</Field>
          <Field label={copy.station.bytesPerChunk}>{summary.bytesPerChunk.toString()}</Field>
          <Field label={copy.station.activeChunks}>
            {formatInteger(summary.activeChunkCount, locale)}
          </Field>
        </Fields>
      </Panel>
    </>
  );
}
