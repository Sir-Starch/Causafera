/**
 * Meridian bar, navigation rail, and the marginalia strip.
 *
 * The meridian is a single strip of equal-height cells divided by hairlines: identity, clock,
 * transport, then the readouts and the session controls. Every control inside it is the same
 * height and carries the same frame, so the bar reads as one instrument rather than as a row
 * of unrelated widgets.
 *
 * Both side panels behave identically: a header with a standard collapse control, a drag
 * handle on the inner edge, and a collapsed state that keeps its expand control visible.
 */

import { copyFor, LOCALE_MARKS, LOCALE_NAMES, LOCALES } from "../i18n";
import { NEGOTIATED_CAPABILITY_NAMES } from "../observer/capability";
import {
  digestPairs,
  formatBytes,
  formatDuration,
  formatInteger,
  type ObserverLocale,
} from "../observer/format";
import { session, useActions, useSession } from "../observer/instance";
import { BATCH_SIZES, type BatchSize } from "../observer/session";
import { AREA_IDS, type AreaId, type WorkspaceState } from "../workspace";
import { AreaMark, Chevron, ResetMark, RunMark, Sigil, StepMark } from "./Sigil";
import { Kbd, Lamp } from "./primitives";
import { Resizer } from "./Resizer";

/* --------------------------------------------------------------- meridian -- */

export function Meridian({ onOpenPalette }: { onOpenPalette(): void }) {
  const state = useSession((current) => ({
    connection: current.connection,
    running: current.running,
    batch: current.batch,
    seed: current.seed,
    locale: current.locale,
    ticks: current.summary?.simulationTicks,
    physicalDigest: current.summary?.physicalDigest,
    historyDigest: current.summary?.historyDigest,
    previousPhysical: current.previous?.physicalDigest,
    previousHistory: current.previous?.historyDigest,
  }));
  const copy = copyFor(state.locale);
  const actions = useActions();
  const attached = state.connection === "connected";

  return (
    <header className="meridian">
      <div className="meridian__cell meridian__cell--brand">
        <span className="meridian__sigil">
          <Sigil size={22} />
        </span>
        <span className="meridian__wordmark">
          <b>{copy.product}</b>
          <span>{copy.observer}</span>
        </span>
      </div>

      <div className="meridian__cell">
        <span className="meridian__key">{copy.transport.ticks}</span>
        <span className="meridian__clock" data-live={state.running}>
          {state.ticks === undefined ? "————" : state.ticks.toString().padStart(4, "0")}
        </span>
      </div>

      <div className="meridian__cell">
        <div className="control-group">
          <button
            type="button"
            className="control control--wide"
            data-active={state.running}
            disabled={!attached}
            onClick={actions.toggleRun}
            title={`${state.running ? copy.transport.pause : copy.transport.run} · Space`}
          >
            <RunMark running={state.running} />
            {state.running ? copy.transport.pause : copy.transport.run}
          </button>
          <button
            type="button"
            className="control"
            disabled={!attached}
            onClick={actions.step}
            title={`${copy.transport.step} · →`}
            aria-label={copy.transport.step}
          >
            <StepMark />
          </button>
        </div>

        <div className="control-group" role="group" aria-label={copy.transport.batch}>
          {BATCH_SIZES.map((size) => (
            <button
              key={size}
              type="button"
              className="control control--compact"
              aria-pressed={state.batch === size}
              onClick={() => session.setBatch(size as BatchSize)}
              title={`${copy.transport.batch}: ${size} ${copy.transport.batchNote}`}
            >
              ×{size}
            </button>
          ))}
        </div>

        {/* Seed only takes effect on reset, so the two travel in one group. */}
        <div className="control-group">
          <label className="control control--field">
            <span className="control__label">{copy.transport.seed}</span>
            <input
              type="number"
              min={0}
              value={state.seed}
              onChange={(event) => session.setSeed(Number(event.target.value) || 0)}
            />
          </label>
          <button
            type="button"
            className="control"
            disabled={!attached}
            onClick={actions.reset}
            title={`${copy.transport.reset} · seed ${state.seed}`}
            aria-label={copy.transport.reset}
          >
            <ResetMark />
          </button>
        </div>
      </div>

      <span className="meridian__spacer" />

      {/* The link only announces itself when it is not simply working. */}
      {!attached && (
        <div className="meridian__cell">
          <Lamp state={state.connection} label={copy.connection[state.connection]} />
        </div>
      )}

      {state.physicalDigest !== undefined && state.historyDigest !== undefined && (
        <div
          className="meridian__cell meridian__cell--optional"
          title={copy.meridian.digestNote}
        >
          <DigestRun
            label="P"
            bytes={state.physicalDigest}
            compare={state.previousPhysical}
            full={digestPairs(state.physicalDigest, 32).join("")}
          />
          <DigestRun
            label="H"
            bytes={state.historyDigest}
            compare={state.previousHistory}
            full={digestPairs(state.historyDigest, 32).join("")}
          />
        </div>
      )}

      <div className="meridian__cell">
        {/*
          Five languages are too many for one button each in a bar this dense, so the switcher
          is a single cell. The options name themselves in their own language: a reader who
          needs the switcher cannot be asked to recognise their language in a foreign one.
        */}
        <div className="control-group">
          <label className="control control--select" title={copy.meridian.locale}>
            <span className="control__label" aria-hidden="true">
              {LOCALE_MARKS[state.locale]}
            </span>
            <select
              value={state.locale}
              aria-label={copy.meridian.locale}
              onChange={(event) => session.setLocale(event.target.value as ObserverLocale)}
            >
              {LOCALES.map((locale) => (
                <option key={locale} value={locale}>
                  {LOCALE_NAMES[locale]}
                </option>
              ))}
            </select>
          </label>
        </div>
        <div className="control-group">
          <button
            type="button"
            className="control"
            onClick={onOpenPalette}
            title={`${copy.meridian.palette} · Ctrl+K`}
            aria-label={copy.meridian.palette}
          >
            ⌘
          </button>
        </div>
      </div>
    </header>
  );
}

/** A digest as one line of byte cells, so it shares the height of every other cell. */
function DigestRun({
  label,
  bytes,
  compare,
  full,
  count = 4,
}: {
  label: string;
  bytes: Uint8Array;
  compare?: Uint8Array;
  full: string;
  count?: number;
}) {
  const cells = [];
  for (let index = 0; index < Math.min(count, bytes.length); index += 1) {
    const value = bytes[index]!;
    const changed = compare !== undefined && compare.length > index && compare[index] !== value;
    cells.push(
      <span key={index} className="digest__byte" data-changed={changed}>
        {value.toString(16).padStart(2, "0")}
      </span>,
    );
  }
  return (
    <span className="digest-run" title={full}>
      <span className="meridian__key">{label}</span>
      {cells}
    </span>
  );
}

/* ------------------------------------------------------------------- rail -- */

export function Rail({
  workspace,
  goTo,
  update,
}: {
  workspace: WorkspaceState;
  goTo(area: AreaId): void;
  update(patch: Partial<WorkspaceState>): void;
}) {
  const state = useSession((current) => ({
    locale: current.locale,
    capabilities: current.negotiation?.capabilities,
  }));
  const copy = copyFor(state.locale);
  const collapsed = workspace.railCollapsed;

  return (
    <nav className="rail" aria-label={copy.observer}>
      <div className="panel-head">
        {!collapsed && <span className="eyebrow">{copy.meridian.areas}</span>}
        <button
          type="button"
          className="control control--quiet"
          aria-expanded={!collapsed}
          onClick={() => update({ railCollapsed: !collapsed })}
          title={collapsed ? copy.meridian.expand : copy.meridian.collapse}
          aria-label={collapsed ? copy.meridian.expand : copy.meridian.collapse}
        >
          <Chevron direction={collapsed ? "right" : "left"} />
        </button>
      </div>

      <div className="rail__nav">
        {AREA_IDS.map((area, index) => (
          <button
            key={area}
            type="button"
            className="rail__item"
            aria-current={workspace.area === area ? "page" : undefined}
            onClick={() => goTo(area)}
            title={`${copy.areas[area].name} · ${index + 1}\n${copy[area].lede}`}
          >
            <span className="rail__index" aria-hidden="true">
              <AreaMark area={area} />
            </span>
            <span className="rail__label">
              <b>{copy.areas[area].name}</b>
              <span>{copy.areas[area].note}</span>
            </span>
          </button>
        ))}
      </div>

      <div className="rail__divider" />

      <div className="rail__register">
        <span className="eyebrow">{copy.instrument.capabilities}</span>
        <ul className="rail__capabilities">
          {(state.capabilities ?? []).map((capability) => (
            <li key={capability} className="numeric">
              <span>— </span>
              {NEGOTIATED_CAPABILITY_NAMES[capability] ?? `#${capability}`}
            </li>
          ))}
          {(state.capabilities === undefined || state.capabilities.length === 0) && (
            <li className="muted">{copy.common.none}</li>
          )}
        </ul>
      </div>

      {!collapsed && (
        <Resizer
          edge="right"
          value={workspace.railWidth}
          min={168}
          max={420}
          onChange={(railWidth) => update({ railWidth })}
          label={copy.meridian.resize}
        />
      )}
    </nav>
  );
}

/* ------------------------------------------------------------ marginalia -- */

export function Marginalia() {
  const state = useSession((current) => ({
    locale: current.locale,
    protocol: current.negotiation?.protocolVersion,
    transport: current.transportLabel,
    replaying: current.replaying,
    exchanges: current.exchanges.length,
    last: current.exchanges[current.exchanges.length - 1],
  }));
  const copy = copyFor(state.locale);

  return (
    <footer className="marginalia">
      <span className="marginalia__item">
        {copy.marginalia.protocol} <b>v{state.protocol ?? "—"}</b>
      </span>
      <span className="marginalia__item">
        {copy.marginalia.transport} <b>{state.transport}</b>
      </span>
      {state.replaying && (
        <span className="marginalia__item" style={{ color: "var(--state-partial)" }}>
          ◆ {copy.marginalia.replay}
        </span>
      )}
      {state.last !== undefined && (
        <span className="marginalia__item">
          {copy.marginalia.lastExchange}{" "}
          <b>
            {state.last.command.replace("observer_", "")} ·{" "}
            {formatBytes(state.last.responseBytes, state.locale)} ·{" "}
            {formatDuration(state.last.durationMs)}
          </b>
        </span>
      )}
      <span className="marginalia__spacer" />
      <span className="marginalia__item">
        {copy.marginalia.exchanges} <b>{formatInteger(state.exchanges, state.locale)}</b>
      </span>
      <span className="marginalia__item">
        <Kbd>Ctrl</Kbd>
        <Kbd>K</Kbd>
      </span>
    </footer>
  );
}
