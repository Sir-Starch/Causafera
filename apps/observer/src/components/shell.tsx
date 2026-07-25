/** Meridian bar, navigation rail, and the marginalia strip. */

import { copyFor, LOCALES } from "../i18n";
import { NEGOTIATED_CAPABILITY_NAMES } from "../observer/capability";
import { digestPairs, formatBytes, formatDuration, formatInteger } from "../observer/format";
import { session, useActions, useSession } from "../observer/instance";
import { BATCH_SIZES, type BatchSize } from "../observer/session";
import { AREA_IDS, type AreaId, type WorkspaceState } from "../workspace";
import { AreaMark, ResetMark, RunMark, Sigil, StepMark } from "./Sigil";
import { DigestPlate, Kbd, Lamp } from "./primitives";

/* --------------------------------------------------------------- meridian -- */

export function Meridian({
  workspace,
  update,
  onOpenPalette,
}: {
  workspace: WorkspaceState;
  update(patch: Partial<WorkspaceState>): void;
  onOpenPalette(): void;
}) {
  const state = useSession(
    (current) => ({
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
      <div className="meridian__brand">
        <span className="meridian__sigil">
          <Sigil />
        </span>
        <span className="meridian__wordmark">
          <b>{copy.product}</b>
          <span>{copy.observer}</span>
        </span>
      </div>

      <div className="meridian__readout meridian__readout--lead">
        <span className="eyebrow">{copy.transport.ticks}</span>
        <span className="meridian__clock" data-live={state.running}>
          {state.ticks === undefined ? "——" : state.ticks.toString().padStart(4, "0")}
        </span>
      </div>

      <div className="meridian__group">
        <div className="transport">
          <button
            type="button"
            className="transport__run"
            data-running={state.running}
            disabled={!attached}
            onClick={actions.toggleRun}
            title={`${state.running ? copy.transport.pause : copy.transport.run} · Space`}
          >
            <RunMark running={state.running} />
            {state.running ? copy.transport.pause : copy.transport.run}
          </button>
          <button
            type="button"
            className="btn btn--ghost btn--icon"
            disabled={!attached}
            onClick={actions.step}
            title={`${copy.transport.step} · →`}
            aria-label={copy.transport.step}
          >
            <StepMark />
          </button>
        </div>

        <div className="segmented" role="group" aria-label={copy.transport.batch}>
          {BATCH_SIZES.map((size) => (
            <button
              key={size}
              type="button"
              className="segmented__option"
              aria-pressed={state.batch === size}
              onClick={() => session.setBatch(size as BatchSize)}
              title={`${copy.transport.batch}: ${size} ${copy.transport.batchNote}`}
            >
              ×{size}
            </button>
          ))}
        </div>

        {/* Seed only takes effect on reset, so the two controls travel together. */}
        <div className="transport transport--reset">
          <label className="input input--seed input--bare">
            <span className="input__label">{copy.transport.seed}</span>
            <input
              type="number"
              min={0}
              value={state.seed}
              onChange={(event) => session.setSeed(Number(event.target.value) || 0)}
            />
          </label>
          <button
            type="button"
            className="btn btn--ghost btn--icon"
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

      {state.physicalDigest !== undefined && state.historyDigest !== undefined && (
        <div className="meridian__readout meridian__readout--optional" title={copy.meridian.digestNote}>
          <DigestPlate
            label={copy.meridian.physicalDigest}
            bytes={state.physicalDigest}
            compare={state.previousPhysical}
            count={4}
            full={digestPairs(state.physicalDigest, 32).join("")}
          />
          <DigestPlate
            label={copy.meridian.historyDigest}
            bytes={state.historyDigest}
            compare={state.previousHistory}
            count={4}
            full={digestPairs(state.historyDigest, 32).join("")}
          />
        </div>
      )}

      <div className="meridian__readout">
        <Lamp state={state.connection} label={copy.connection[state.connection]} />
      </div>

      <div className="meridian__group">
        <div
          className="segmented segmented--text meridian__locale"
          role="group"
          aria-label={copy.meridian.locale}
        >
          {LOCALES.map((locale) => (
            <button
              key={locale}
              type="button"
              className="segmented__option"
              aria-pressed={state.locale === locale}
              onClick={() => session.setLocale(locale)}
            >
              {locale.slice(0, 2).toUpperCase()}
            </button>
          ))}
        </div>
        <button
          type="button"
          className="btn btn--ghost btn--icon"
          onClick={onOpenPalette}
          title={`${copy.meridian.palette} · Ctrl+K`}
          aria-label={copy.meridian.palette}
        >
          ⌘
        </button>
        <button
          type="button"
          className="btn btn--ghost btn--icon"
          aria-pressed={workspace.dockOpen}
          onClick={() => update({ dockOpen: !workspace.dockOpen })}
          title={`${copy.meridian.inspector} · I`}
          aria-label={copy.meridian.inspector}
        >
          ▤
        </button>
      </div>
    </header>
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
  const state = useSession(
    (current) => ({
      locale: current.locale,
      capabilities: current.negotiation?.capabilities,
  }));
  const copy = copyFor(state.locale);

  return (
    <nav className="rail" aria-label={copy.observer}>
      <div className="rail__nav">
        {AREA_IDS.map((area, index) => (
          <button
            key={area}
            type="button"
            className="rail__item"
            aria-current={workspace.area === area ? "page" : undefined}
            onClick={() => goTo(area)}
            title={`${copy.areas[area].name} · ${index + 1}`}
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
        <ul style={{ display: "flex", flexDirection: "column", gap: 2, paddingTop: 6 }}>
          {(state.capabilities ?? []).map((capability) => (
            <li key={capability} className="numeric" style={{ fontSize: "var(--t-micro)", color: "var(--ink-faint)" }}>
              <span style={{ color: "var(--sig-trace)" }}>▸ </span>
              {NEGOTIATED_CAPABILITY_NAMES[capability] ?? `#${capability}`}
            </li>
          ))}
          {(state.capabilities === undefined || state.capabilities.length === 0) && (
            <li className="muted" style={{ fontSize: "var(--t-micro)" }}>
              {copy.common.none}
            </li>
          )}
        </ul>
      </div>

      <div className="rail__foot">
        <button
          type="button"
          className="btn btn--ghost"
          onClick={() => update({ railCollapsed: !workspace.railCollapsed })}
        >
          {workspace.railCollapsed ? "»" : "«"}
        </button>
      </div>
    </nav>
  );
}

/* ------------------------------------------------------------ marginalia -- */

export function Marginalia() {
  const state = useSession(
    (current) => ({
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
