/**
 * Instrument — protocol state and the honest coverage register.
 *
 * Two things belong here that no other area can carry: what the transport is actually doing
 * (real exchanges, real byte counts, real durations), and what this instrument cannot see
 * yet. Presenting the second is a design requirement, not an apology: a scientific
 * instrument states its range.
 */

import { useMemo, useState } from "react";

import {
  Division,
  Field,
  Fields,
  MaturityPips,
  Panel,
  Tag,
  Unsurveyed,
} from "../components/primitives";
import {
  CAPABILITY_REGISTER,
  NEGOTIATED_CAPABILITY_NAMES,
  capabilityCounts,
  type CapabilityState,
} from "../observer/capability";
import { formatBytes, formatDuration, formatInteger } from "../observer/format";
import { useCopy, useSession } from "../observer/instance";
import { EXCHANGE_CAPACITY, HISTORY_CAPACITY } from "../observer/session";
import type { StatusTone } from "../observer/claims";
import type { AreaProps } from "../workspace";

const STATE_TONE: Record<CapabilityState, StatusTone | "live" | "quiet"> = {
  live: "supported",
  bounded: "partial",
  "absent-projection": "unknown",
  "absent-domain": "unknown",
};

export function InstrumentArea({ goTo }: AreaProps) {
  const copy = useCopy();
  const locale = useSession((state) => state.locale);
  const negotiation = useSession((state) => state.negotiation);
  const transportLabel = useSession((state) => state.transportLabel);
  const replaying = useSession((state) => state.replaying);
  const exchanges = useSession((state) => state.exchanges);
  const world = useSession((state) => state.world);
  const [filter, setFilter] = useState<CapabilityState | "all">("all");

  const counts = useMemo(() => capabilityCounts(), []);
  const recent = useMemo(() => [...exchanges].reverse(), [exchanges]);

  const stateLabel: Record<CapabilityState, string> = {
    live: copy.instrument.stateLive,
    bounded: copy.instrument.stateBounded,
    "absent-projection": copy.instrument.stateAbsentProjection,
    "absent-domain": copy.instrument.stateAbsentDomain,
  };

  const filters: (CapabilityState | "all")[] = [
    "all",
    "live",
    "bounded",
    "absent-projection",
    "absent-domain",
  ];

  return (
    <>
      <div className="area-head">
        <div className="area-head__titles">
          <span className="eyebrow">{copy.instrument.eyebrow}</span>
          <h1 className="display">{copy.instrument.title}</h1>
          <p className="lede">{copy.instrument.lede}</p>
        </div>
      </div>

      <div className="grid grid--halves">
        <Panel title={copy.instrument.negotiation} eyebrow={copy.marginalia.protocol}>
          <Fields>
            <Field label={copy.instrument.protocolVersion}>
              v{negotiation?.protocolVersion ?? "—"}
            </Field>
            <Field label={copy.instrument.channel} text>
              {transportLabel}
              {replaying && (
                <>
                  {" "}
                  <Tag tone="partial">{copy.marginalia.replay}</Tag>
                </>
              )}
            </Field>
            <Field label={copy.instrument.timeAtConnect}>
              {negotiation?.timeAtConnect.toString() ?? "—"}
            </Field>
            <Field label={copy.instrument.capabilities} stacked>
              <span className="trace-chips">
                {(negotiation?.capabilities ?? []).map((capability) => (
                  <span key={capability} className="trace-chip trace-chip--static">
                    {NEGOTIATED_CAPABILITY_NAMES[capability] ?? `#${capability}`}
                  </span>
                ))}
                {(negotiation?.capabilities ?? []).length === 0 && (
                  <span className="muted">{copy.common.none}</span>
                )}
              </span>
            </Field>
          </Fields>
        </Panel>

        <Panel title={copy.instrument.boundsTitle} eyebrow={copy.common.bounded} lede={copy.instrument.boundsLede}>
          <Fields>
            <Field label={copy.instrument.boundAdvance}>64</Field>
            <Field label={copy.instrument.boundDeltas}>
              {world?.materialSurfaceDeltas.length ?? 0} / 64
            </Field>
            <Field label={copy.instrument.boundHistory}>{HISTORY_CAPACITY}</Field>
            <Field label={copy.instrument.boundExchanges}>{EXCHANGE_CAPACITY}</Field>
            <Field label={copy.instrument.boundStreams} text>
              1 <span className="muted">· {copy.instrument.boundStreamsNote}</span>
            </Field>
          </Fields>
        </Panel>
      </div>

      <Panel
        title={copy.instrument.log}
        eyebrow={`${exchanges.length} / ${EXCHANGE_CAPACITY}`}
        lede={copy.instrument.logLede}
        flushBody
      >
        {recent.length === 0 ? (
          <div style={{ padding: "var(--s3)" }}>
            <Unsurveyed title={copy.instrument.noExchanges} />
          </div>
        ) : (
          <div className="table-frame" style={{ ["--table-height" as string]: "18rem" }}>
            <table className="data">
              <thead>
                <tr>
                  <th className="num">#</th>
                  <th>{copy.instrument.command}</th>
                  <th>{copy.instrument.detail}</th>
                  <th className="num">{copy.instrument.request}</th>
                  <th className="num">{copy.instrument.response}</th>
                  <th className="num">{copy.instrument.duration}</th>
                  <th>{copy.instrument.outcome}</th>
                </tr>
              </thead>
              <tbody>
                {recent.map((exchange) => (
                  <tr key={exchange.id}>
                    <td className="num muted">{exchange.id}</td>
                    <td className="numeric emphasis">{exchange.command.replace("observer_", "")}</td>
                    <td className="muted">{exchange.detail ?? "—"}</td>
                    <td className="num">
                      {exchange.requestBytes === 0
                        ? "—"
                        : formatBytes(exchange.requestBytes, locale)}
                    </td>
                    <td className="num">{formatBytes(exchange.responseBytes, locale)}</td>
                    <td className="num">{formatDuration(exchange.durationMs)}</td>
                    <td>
                      {exchange.outcome === "ok" ? (
                        <Tag tone="supported" dot>
                          {copy.instrument.ok}
                        </Tag>
                      ) : (
                        <Tag tone="unsupported" dot>
                          {copy.instrument.failed}
                        </Tag>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Panel>

      <Panel
        title={copy.instrument.register}
        eyebrow={`${counts.live + counts.bounded} / ${
          counts.live + counts.bounded + counts["absent-projection"] + counts["absent-domain"]
        }`}
        lede={copy.instrument.registerLede}
        tools={
          <div className="segmented segmented--text" role="group" aria-label={copy.instrument.register}>
            {filters.map((candidate) => (
              <button
                key={candidate}
                type="button"
                className="segmented__option"
                aria-pressed={filter === candidate}
                onClick={() => setFilter(candidate)}
              >
                {candidate === "all"
                  ? `${copy.common.total} ${CAPABILITY_REGISTER.reduce((sum, group) => sum + group.entries.length, 0)}`
                  : `${stateLabel[candidate]} ${counts[candidate]}`}
              </button>
            ))}
          </div>
        }
      >
        {CAPABILITY_REGISTER.map((group) => {
          const entries = group.entries.filter(
            (entry) => filter === "all" || entry.state === filter,
          );
          if (entries.length === 0) return null;
          return (
            <section key={group.id}>
              <Division>{group.title[locale]}</Division>
              <div className="register">
                {entries.map((entry) => (
                  <article
                    key={entry.id}
                    className="register__row"
                    style={{ gridTemplateColumns: "minmax(0, 1fr) auto", alignItems: "start" }}
                  >
                    <div>
                      <div className="register__primary">
                        <span className="register__name" style={{ color: "var(--ink)" }}>
                          {entry.title[locale]}
                        </span>
                        {entry.area !== undefined && (
                          <button
                            type="button"
                            className="trace-chip"
                            onClick={() => goTo(entry.area!)}
                          >
                            {copy.areas[entry.area].name} →
                          </button>
                        )}
                      </div>
                      <p className="lede" style={{ marginTop: 2 }}>
                        {entry.detail[locale]}
                      </p>
                    </div>
                    <div
                      style={{
                        display: "flex",
                        flexDirection: "column",
                        alignItems: "flex-end",
                        gap: 4,
                      }}
                    >
                      <Tag tone={STATE_TONE[entry.state]}>{stateLabel[entry.state]}</Tag>
                      <span
                        className="register__sub"
                        title={`${copy.instrument.domainMaturity} M${entry.domainMaturity} · ${copy.instrument.observerMaturity} M${entry.observerMaturity}`}
                        style={{ display: "flex", gap: 6, alignItems: "center" }}
                      >
                        <MaturityPips level={entry.domainMaturity} />
                        <span className="muted">/</span>
                        <MaturityPips level={entry.observerMaturity} />
                      </span>
                    </div>
                  </article>
                ))}
              </div>
            </section>
          );
        })}
      </Panel>
    </>
  );
}

export function InstrumentDock() {
  const copy = useCopy();
  const locale = useSession((state) => state.locale);
  const exchanges = useSession((state) => state.exchanges);

  const stats = useMemo(() => {
    if (exchanges.length === 0) return undefined;
    const durations = exchanges.map((exchange) => exchange.durationMs).sort((a, b) => a - b);
    const bytes = exchanges.reduce((sum, exchange) => sum + exchange.responseBytes, 0);
    const failures = exchanges.filter((exchange) => exchange.outcome === "failed").length;
    return {
      median: durations[Math.floor(durations.length / 2)] ?? 0,
      slowest: durations[durations.length - 1] ?? 0,
      bytes,
      failures,
    };
  }, [exchanges]);

  const byCommand = useMemo(() => {
    const counts = new Map<string, number>();
    for (const exchange of exchanges) {
      const key = exchange.command.replace("observer_", "");
      counts.set(key, (counts.get(key) ?? 0) + 1);
    }
    return [...counts.entries()].sort((a, b) => b[1] - a[1]);
  }, [exchanges]);

  if (stats === undefined) {
    return <Unsurveyed title={copy.instrument.noExchanges} />;
  }

  return (
    <>
      <Panel variant="flush" title={copy.instrument.log} eyebrow={copy.common.derived}>
        <Fields>
          <Field label={copy.instrument.duration}>{formatDuration(stats.median)}</Field>
          <Field label={`${copy.instrument.duration} · ${copy.common.peak}`}>
            {formatDuration(stats.slowest)}
          </Field>
          <Field label={copy.instrument.response}>{formatBytes(stats.bytes, locale)}</Field>
          <Field label={copy.instrument.failed}>{formatInteger(stats.failures, locale)}</Field>
        </Fields>
      </Panel>

      <Panel variant="flush" title={copy.instrument.command} eyebrow={copy.marginalia.exchanges}>
        <Fields>
          {byCommand.map(([command, count]) => (
            <Field key={command} label={command}>
              {count}
            </Field>
          ))}
        </Fields>
      </Panel>
    </>
  );
}
