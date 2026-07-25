/**
 * Flux — causal activity over run time.
 *
 * Three readings of the same run: rate recorders over the observer-side series, the surface
 * condition ladder with its provenance markers, and the bounded transition ledger. A trace
 * anchor selected anywhere here filters the ledger, which is the only provenance navigation
 * the current protocol supports — there is no ancestry query yet, and the interface says so
 * rather than implying a graph it cannot draw.
 */

import { useMemo } from "react";

import {
  Derived,
  Field,
  Fields,
  Panel,
  RateTable,
  Readout,
  Tag,
  TraceChip,
  Unsurveyed,
} from "../components/primitives";
import { formatCompact, formatInteger } from "../observer/format";
import { useCopy, useFeed, useSession } from "../observer/instance";
import {
  activityRates,
  buildGates,
  buildLadders,
  buildLedger,
  levelSeries,
  rateSeries,
  type SignalId,
  type SurfaceLadder,
  type TraceRole,
} from "../observer/models";
import { ConditionLadder } from "../viz/ConditionLadder";
import { ChartRecorder } from "../viz/ChartRecorder";
import type { AreaProps } from "../workspace";

function ladderLabel(ladder: SurfaceLadder): string {
  return `${ladder.chunkX}, ${ladder.chunkY}, ${ladder.chunkZ}·${ladder.cellOrdinal}`;
}

export function FluxArea({ workspace, update }: AreaProps) {
  useFeed("world");
  const copy = useCopy();
  const locale = useSession((state) => state.locale);
  const history = useSession((state) => state.history);
  const world = useSession((state) => state.world);
  const summary = useSession((state) => state.summary);

  const ladders = useMemo(() => buildLadders(world?.materialSurfaceDeltas ?? []), [world]);
  const gates = useMemo(() => buildGates(world?.materialSurfaceGateDeltas ?? []), [world]);
  const rates = useMemo(() => activityRates(history), [history]);

  const ledger = useMemo(() => {
    const all = buildLedger(world?.materialSurfaceDeltas ?? []);
    const filter = workspace.traceFilter;
    if (filter === undefined) return all;
    return all.filter((entry) => entry.traces.some((trace) => trace.id === filter));
  }, [workspace.traceFilter, world]);

  const traceRates = useMemo(
    () => [
      rateSeries(history, "traceRate", "trace", (item) => item.causalTraceCount),
      rateSeries(history, "manaCellRate", "mana", (item) => item.manaCellChanges),
    ],
    [history],
  );
  const actionRates = useMemo(
    () => [
      rateSeries(history, "committedRate", "life", (item) => item.actorActionsCommitted),
      rateSeries(history, "rejectedRate", "refused", (item) => item.actorActionsRejected),
    ],
    [history],
  );
  const resolutionSeries = useMemo(
    () => [
      levelSeries(history, "relevance", "resolution", (item) => item.resolutionRelevance),
    ],
    [history],
  );

  const rateRows: { id: string; label: string; signal: SignalId }[] = [
    { id: "traces", label: copy.station.traces, signal: "trace" },
    { id: "physicalEvents", label: copy.station.physicalEvents, signal: "physical" },
    { id: "manaCellChanges", label: copy.station.manaCellChanges, signal: "mana" },
    { id: "actorActionsCommitted", label: copy.station.committed, signal: "life" },
    { id: "actorActionsRejected", label: copy.station.rejected, signal: "refused" },
    { id: "resolutionTransitions", label: copy.station.resolutionTransitions, signal: "resolution" },
  ];

  return (
    <>
      <div className="area-head">
        <div className="area-head__titles">
          <span className="eyebrow">{copy.flux.eyebrow}</span>
          <h1 className="display">{copy.flux.title}</h1>
          <p className="lede">{copy.flux.lede}</p>
        </div>
        <div className="area-head__meta">
          <Readout
            label={copy.flux.rate}
            value={
              rates.get("traces")?.available === true
                ? rates.get("traces")!.perTick.toFixed(1)
                : "—"
            }
            unit={`${copy.station.traces.toLowerCase()}${copy.flux.perTick}`}
            signal="trace"
            size="hero"
            align="end"
          />
        </div>
      </div>

      <div className="grid grid--thirds">
        <Panel
          variant="flush"
          title={copy.flux.recorder}
          eyebrow={copy.common.derived}
          lede={copy.flux.ratesLede}
        >
          <ChartRecorder
            series={traceRates}
            height={144}
            label={copy.flux.recorder}
            tickLabel={copy.transport.ticks}
            legendLabels={{
              traceRate: copy.station.traces,
              manaCellRate: copy.station.manaCellChanges,
            }}
            emptyLabel={copy.survey.noWorld}
            valueFormat={(value) => value.toFixed(0)}
          />
        </Panel>

        <Panel
          variant="flush"
          title={copy.station.admission}
          eyebrow={copy.common.derived}
          lede={copy.station.admissionLede}
        >
          <ChartRecorder
            series={actionRates}
            height={144}
            label={copy.station.admission}
            tickLabel={copy.transport.ticks}
            legendLabels={{
              committedRate: copy.station.committed,
              rejectedRate: copy.station.rejected,
            }}
            emptyLabel={copy.survey.noWorld}
            valueFormat={(value) => value.toFixed(1)}
          />
        </Panel>

        <Panel
          variant="flush"
          title={copy.station.resolutionRelevance}
          eyebrow={copy.common.projection}
          lede={copy.survey.resolution}
        >
          <ChartRecorder
            series={resolutionSeries}
            height={144}
            label={copy.station.resolutionRelevance}
            tickLabel={copy.transport.ticks}
            legendLabels={{ relevance: copy.station.resolutionRelevance }}
            emptyLabel={copy.survey.noWorld}
            fillFirst
          />
          <p className="chart__caption">
            {copy.station.resolutionLevel}:{" "}
            <b className="numeric">{summary === undefined ? "—" : summary.resolutionLevel}</b>
          </p>
        </Panel>
      </div>

      <Panel
        title={copy.flux.ladder}
        eyebrow={copy.common.bounded}
        lede={copy.flux.ladderLede}
        tools={
          workspace.selectedSurface === undefined ? undefined : (
            <button
              type="button"
              className="btn btn--ghost"
              onClick={() => update({ selectedSurface: undefined })}
            >
              {copy.flux.clearSelection}
            </button>
          )
        }
        foot={
          <div className="ladder__key">
            <span className="ladder__key-item">
              <span className="glyph glyph--contact" /> {copy.flux.contact}
            </span>
            <span className="ladder__key-item">
              <span className="glyph glyph--mana" /> {copy.flux.manaEffect}
            </span>
            <span className="ladder__key-item">
              <span className="glyph glyph--gate" /> {copy.flux.localMana}
            </span>
          </div>
        }
      >
        {ladders.length === 0 ? (
          <Unsurveyed title={copy.flux.noLadder}>{copy.flux.noLadderBody}</Unsurveyed>
        ) : (
          <ConditionLadder
            ladders={ladders}
            selectedKey={workspace.selectedSurface}
            onSelect={(key) => update({ selectedSurface: key === "" ? undefined : key })}
            ariaLabel={copy.flux.ladder}
            tickLabel={copy.transport.ticks}
            conditionLabel={copy.flux.condition}
            labelFor={ladderLabel}
            probeLabels={{
              condition: copy.flux.condition,
              mana: copy.survey.mana,
              localMana: copy.flux.localMana,
              contact: copy.flux.contact,
            }}
          />
        )}
      </Panel>

      <div className="grid grid--wide-left">
        <Panel
          title={copy.flux.ledger}
          eyebrow={`${ledger.length} ${copy.common.of} ${world?.materialSurfaceDeltas.length ?? 0}`}
          lede={copy.flux.ledgerLede}
          flushBody
          tools={
            workspace.traceFilter === undefined ? (
              <Tag tone="quiet">{copy.common.bounded}</Tag>
            ) : (
              <button
                type="button"
                className="btn"
                onClick={() => update({ traceFilter: undefined })}
              >
                {copy.flux.clearFilter} · #{workspace.traceFilter.toString()}
              </button>
            )
          }
        >
          {ledger.length === 0 ? (
            <div style={{ padding: "var(--s3)" }}>
              <Unsurveyed title={copy.flux.noLadder}>{copy.flux.noLadderBody}</Unsurveyed>
            </div>
          ) : (
            <div className="table-frame" style={{ ["--table-height" as string]: "24rem" }}>
              <table className="data">
                <thead>
                  <tr>
                    <th className="num">{copy.flux.tick}</th>
                    <th>{copy.flux.surface}</th>
                    <th className="num">{copy.flux.condition}</th>
                    <th className="num">{copy.survey.mana}</th>
                    <th className="num">{copy.flux.localMana}</th>
                    <th>{copy.assay.traces}</th>
                  </tr>
                </thead>
                <tbody>
                  {ledger.map((entry) => (
                    <tr
                      key={entry.id}
                      data-selected={workspace.selectedSurface === entry.surface}
                      onClick={() => update({ selectedSurface: entry.surface })}
                      style={{ cursor: "pointer" }}
                    >
                      <td className="num emphasis">{entry.tick}</td>
                      <td className="numeric">
                        {entry.chunkX}, {entry.chunkY}, {entry.chunkZ} · #{entry.cellOrdinal}
                      </td>
                      <td className="num">
                        {entry.beforeCondition} → {entry.afterCondition}
                      </td>
                      <td className="num" style={{ color: entry.manaTotal > 0 ? "var(--sig-mana)" : undefined }}>
                        {entry.manaTotal === 0 ? "—" : formatCompact(entry.manaTotal, locale)}
                      </td>
                      <td className="num">
                        {entry.localManaAfter === undefined
                          ? "—"
                          : `${entry.localManaBefore} → ${entry.localManaAfter}`}
                      </td>
                      <td>
                        <span className="trace-chips">
                          {entry.traces.map((trace) => (
                            <TraceChip
                              key={trace.id.toString()}
                              id={trace.id}
                              active={workspace.traceFilter === trace.id}
                              title={trace.roles.map((role) => roleLabel(role, copy)).join(" · ")}
                              onSelect={(id) =>
                                update({ traceFilter: workspace.traceFilter === id ? undefined : id })
                              }
                            />
                          ))}
                        </span>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </Panel>

        <div style={{ display: "flex", flexDirection: "column", gap: "var(--s4)" }}>
          <Panel variant="flush" title={copy.flux.rates} eyebrow={copy.common.derived}>
            <RateTable
              columns={{
                quantity: copy.flux.quantity,
                rate: copy.flux.perTick,
                total: copy.flux.total,
              }}
              rows={rateRows.map((row) => {
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
            <p className="chart__caption" style={{ marginTop: "var(--s3)" }}>
              <Derived>{copy.marginalia.derivedHistory}</Derived>
            </p>
          </Panel>

          <Panel variant="flush" title={copy.flux.gates} eyebrow={copy.common.bounded} lede={copy.flux.gatesLede}>
            {gates.length === 0 ? (
              <Unsurveyed title={copy.flux.noGates}>{copy.flux.noGatesBody}</Unsurveyed>
            ) : (
              <Fields>
                {gates.map((gate) => (
                  <Field
                    key={gate.key}
                    label={`${copy.flux.tick} ${gate.tick} · ${gate.chunkX} ${gate.chunkY} ${gate.chunkZ}`}
                    stacked
                  >
                    {gate.beforeActive ? copy.flux.gateOpen : copy.flux.gateClosed} →{" "}
                    {gate.afterActive ? copy.flux.gateOpen : copy.flux.gateClosed} ·{" "}
                    {gate.localManaBefore} → {gate.localManaAfter}
                    <span className="trace-chips" style={{ marginTop: 4 }}>
                      <TraceChip
                        id={gate.gateTransitionTraceId}
                        onSelect={(id) => update({ traceFilter: id })}
                      />
                      <TraceChip
                        id={gate.localManaTransitionTraceId}
                        onSelect={(id) => update({ traceFilter: id })}
                      />
                    </span>
                  </Field>
                ))}
              </Fields>
            )}
          </Panel>
        </div>
      </div>
    </>
  );
}

function roleLabel(role: TraceRole, copy: ReturnType<typeof useCopy>): string {
  if (role === "contact") return copy.flux.contact;
  if (role === "manaEffect") return copy.flux.manaEffect;
  if (role === "manaTransition") return copy.flux.manaTransition;
  return copy.flux.localMana;
}

export function FluxDock({ workspace, update }: AreaProps) {
  const copy = useCopy();
  const locale = useSession((state) => state.locale);
  const world = useSession((state) => state.world);
  const ladders = useMemo(() => buildLadders(world?.materialSurfaceDeltas ?? []), [world]);
  const ladder = ladders.find((entry) => entry.key === workspace.selectedSurface);

  if (ladder === undefined) {
    return (
      <>
        <Unsurveyed title={copy.flux.surface}>{copy.flux.ladderLede}</Unsurveyed>
        <Panel variant="flush" title={copy.instrument.boundsTitle} eyebrow={copy.common.bounded}>
          <Fields>
            <Field label={copy.flux.surface}>{ladders.length}</Field>
            <Field label={copy.instrument.boundDeltas}>
              {world?.materialSurfaceDeltas.length ?? 0} / 64
            </Field>
          </Fields>
        </Panel>
      </>
    );
  }

  const last = ladder.steps[ladder.steps.length - 1];

  return (
    <>
      <Panel
        variant="flush"
        eyebrow={copy.flux.surface}
        title={`${ladder.chunkX}, ${ladder.chunkY}, ${ladder.chunkZ} · #${ladder.cellOrdinal}`}
        tools={
          <button
            type="button"
            className="btn btn--ghost"
            onClick={() => update({ selectedSurface: undefined })}
          >
            {copy.common.close}
          </button>
        }
      >
        <Fields>
          <Field label={copy.survey.chart}>#{ladder.chartId.toString()}</Field>
          <Field label={copy.survey.surfaceCell}>
            {ladder.cell.x}, {ladder.cell.y}, {ladder.cell.z}
          </Field>
          <Field label={copy.survey.transitions}>{ladder.steps.length}</Field>
          <Field label={copy.flux.condition}>
            {ladder.minCondition} → {ladder.maxCondition}
          </Field>
          <Field label={copy.transport.ticks}>
            {ladder.firstTick} … {ladder.lastTick}
          </Field>
          <Field label={copy.flux.manaEffect}>{ladder.manaEffects}</Field>
          <Field label={copy.flux.localMana}>{ladder.localManaEvents}</Field>
          {last !== undefined && (
            <Field label={copy.survey.mana}>{formatInteger(last.manaTotal, locale)}</Field>
          )}
        </Fields>
      </Panel>

      <Panel variant="flush" title={copy.flux.ledger} eyebrow={copy.common.bounded} flushBody>
        <div className="table-frame" style={{ ["--table-height" as string]: "20rem" }}>
          <table className="data">
            <thead>
              <tr>
                <th className="num">{copy.flux.tick}</th>
                <th className="num">{copy.flux.condition}</th>
                <th>{copy.assay.traces}</th>
              </tr>
            </thead>
            <tbody>
              {[...ladder.steps].reverse().map((step, index) => (
                <tr key={`${step.tick}:${index}`}>
                  <td className="num emphasis">{step.tick}</td>
                  <td className="num">
                    {step.beforeCondition} → {step.afterCondition}
                  </td>
                  <td>
                    <span className="trace-chips">
                      {step.contactTraceId !== undefined && (
                        <TraceChip
                          id={step.contactTraceId}
                          title={copy.flux.contact}
                          onSelect={(id) => update({ traceFilter: id })}
                        />
                      )}
                      {step.manaEffectTraceId !== undefined && (
                        <TraceChip
                          id={step.manaEffectTraceId}
                          title={copy.flux.manaEffect}
                          onSelect={(id) => update({ traceFilter: id })}
                        />
                      )}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Panel>
    </>
  );
}
