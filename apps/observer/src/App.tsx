import { useMemo, useState } from "react";
import { digestHex, type SpatialChunkSummary } from "@ontopolis/observer-protocol";

import { CausalFlow } from "./components/CausalFlow";
import { ConnectionStatus } from "./components/ConnectionStatus";
import { ExplanationPanel } from "./components/ExplanationPanel";
import { InspectorPanel } from "./components/InspectorPanel";
import { MetricCard } from "./components/MetricCard";
import { SimulationControls } from "./components/SimulationControls";
import { TimelinePanel } from "./components/TimelinePanel";
import { WorldViewport } from "./components/WorldViewport";
import { copyFor } from "./i18n";
import { chunkKey, useObserverSession } from "./useObserverSession";

type ViewId = "world" | "causality" | "explanation";

export function App() {
  const [view, setView] = useState<ViewId>("world");
  const session = useObserverSession(view === "world");
  const copy = copyFor(session.locale);
  const [selectedKey, setSelectedKey] = useState<string>();
  const selectedChunk = useMemo(
    () => session.world?.chunks.find((chunk) => chunkKey(chunk) === selectedKey),
    [selectedKey, session.world],
  );
  const navItems: { id: ViewId; label: string }[] = [
    { id: "world", label: copy.world },
    { id: "causality", label: copy.causality },
    { id: "explanation", label: copy.explanation },
  ];

  const selectChunk = (chunk: SpatialChunkSummary) => {
    setSelectedKey(chunkKey(chunk));
  };

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="wordmark">
          <span className="wordmark-mark" aria-hidden="true">O</span>
          <div>
            <strong>{copy.product}</strong>
            <span>{copy.observer}</span>
          </div>
        </div>

        <nav className="primary-nav" aria-label={copy.observer}>
          {navItems.map((item, index) => (
            <button
              key={item.id}
              type="button"
              className={view === item.id ? "is-active" : undefined}
              aria-current={view === item.id ? "page" : undefined}
              onClick={() => setView(item.id)}
            >
              <span className="nav-index numeric">{String(index + 1).padStart(2, "0")}</span>
              {item.label}
            </button>
          ))}
        </nav>

        <SimulationControls
          connection={session.connection}
          isPlaying={session.isPlaying}
          playbackRate={session.playbackRate}
          copy={copy}
          onTogglePlayback={session.togglePlayback}
          onStep={session.step}
          onReset={session.reset}
          onPlaybackRate={session.setPlaybackRate}
        />

        <div className="sidebar-footer">
          <span>{copy.protocol} v1</span>
          <div className="locale-control" aria-label="Language">
            <button
              type="button"
              className={session.locale === "ru-RU" ? "is-active" : undefined}
              aria-pressed={session.locale === "ru-RU"}
              onClick={() => session.setLocale("ru-RU")}
            >
              RU
            </button>
            <button
              type="button"
              className={session.locale === "en-US" ? "is-active" : undefined}
              aria-pressed={session.locale === "en-US"}
              onClick={() => session.setLocale("en-US")}
            >
              EN
            </button>
          </div>
        </div>
      </aside>

      <div className="workspace">
        <header className="topbar">
          <ConnectionStatus
            connection={session.connection}
            isPlaying={session.isPlaying}
            ticks={session.summary?.simulationTicks}
            copy={copy}
          />
          <div className="topbar-digests">
            {session.summary !== undefined && (
              <>
                <span title={digestHex(session.summary.physicalDigest)}>
                  P · <b className="numeric">{digestHex(session.summary.physicalDigest, 4)}</b>
                </span>
                <span title={digestHex(session.summary.historyDigest)}>
                  H · <b className="numeric">{digestHex(session.summary.historyDigest, 4)}</b>
                </span>
              </>
            )}
          </div>
        </header>

        {session.error !== undefined && <div className="error-banner" role="alert">{session.error}</div>}

        <main className="main-content">
          <section className="view-content">
            <div className="view-heading">
              <div>
                <span className="eyebrow">{copy.objectiveProjection}</span>
                <h1>{navItems.find((item) => item.id === view)?.label}</h1>
              </div>
              <p>{copy.projectionNotice}</p>
            </div>

            {view === "world" && (
              <>
                <WorldViewport
                  world={session.world}
                  selectedKey={selectedKey}
                  copy={copy}
                  onSelect={selectChunk}
                />
                <TimelinePanel history={session.history} copy={copy} />
              </>
            )}
            {view === "causality" && (
              <>
                <CausalFlow summary={session.summary} copy={copy} />
                <TimelinePanel history={session.history} copy={copy} />
              </>
            )}
            {view === "explanation" && (
              <ExplanationPanel
                report={session.explanation}
                isAnalyzing={session.isAnalyzing}
                locale={session.locale}
                copy={copy}
                onAnalyze={session.analyze}
              />
            )}
          </section>

          <aside className="detail-rail">
            <div className="metric-grid">
              <MetricCard label={copy.population} value={session.summary?.populationTotal.toString() ?? "—"} />
              <MetricCard label={copy.actors} value={session.summary?.actorCount.toString() ?? "—"} />
              <MetricCard label={copy.mana} value={session.summary?.manaTotal.toString() ?? "—"} detail={`max ${session.summary?.manaMaximum.toString() ?? "—"}`} />
              <MetricCard label={copy.traces} value={session.summary?.causalTraceCount.toString() ?? "—"} />
            </div>

            <InspectorPanel chunk={selectedChunk} copy={copy} />

            <section className="panel digest-panel">
              <div className="panel-heading">
                <span className="eyebrow">{copy.digest}</span>
                <h2>Schema {session.summary?.digestSchemaVersion ?? "—"}</h2>
              </div>
              <dl>
                <div>
                  <dt>{copy.physicalDigest}</dt>
                  <dd className="numeric">{session.summary === undefined ? "—" : digestHex(session.summary.physicalDigest, 8)}</dd>
                </div>
                <div>
                  <dt>{copy.historyDigest}</dt>
                  <dd className="numeric">{session.summary === undefined ? "—" : digestHex(session.summary.historyDigest, 8)}</dd>
                </div>
              </dl>
            </section>
          </aside>
        </main>
      </div>
    </div>
  );
}
