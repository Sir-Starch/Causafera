/**
 * The observer shell.
 *
 * A meridian bar carrying run identity and transport, a navigation rail, one area workspace,
 * a context inspector dock, and a marginalia strip reporting the live transport. The layout
 * reorganises rather than shrinking: below 68rem the rail collapses to marks and the dock
 * becomes an overlay; below 52rem the brand and digest plates give up their space first.
 */

import { useCallback, useEffect, useMemo, useState } from "react";

import { AssayArea, AssayDock } from "./areas/AssayArea";
import { ChartArea, ChartDock } from "./areas/ChartArea";
import { FluxArea, FluxDock } from "./areas/FluxArea";
import { InstrumentArea, InstrumentDock } from "./areas/InstrumentArea";
import { StationArea, StationDock } from "./areas/StationArea";
import { CommandPalette } from "./components/CommandPalette";
import { Notice } from "./components/primitives";
import { Marginalia, Meridian, Rail } from "./components/shell";
import { TerraIncognita } from "./components/TerraIncognita";
import { Unattached } from "./components/Unattached";
import { session, useCopy, useSession } from "./observer/instance";
import { AREA_IDS, INITIAL_WORKSPACE, type AreaId, type AreaProps, type WorkspaceState } from "./workspace";

const AREAS: Record<
  AreaId,
  { view: (props: AreaProps) => JSX.Element; dock: (props: AreaProps) => JSX.Element | null }
> = {
  station: { view: StationArea, dock: StationDock },
  chart: { view: ChartArea, dock: ChartDock },
  flux: { view: FluxArea, dock: FluxDock },
  assay: { view: AssayArea, dock: AssayDock },
  instrument: { view: InstrumentArea, dock: InstrumentDock },
};

export function App() {
  const [workspace, setWorkspace] = useState<WorkspaceState>(INITIAL_WORKSPACE);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const copy = useCopy();
  const connection = useSession((state) => state.connection);
  const error = useSession((state) => state.error);
  const hasSummary = useSession((state) => state.summary !== undefined);

  useEffect(() => {
    session.start();
  }, []);

  const update = useCallback((patch: Partial<WorkspaceState>) => {
    setWorkspace((current) => ({ ...current, ...patch }));
  }, []);

  const goTo = useCallback(
    (area: AreaId) => {
      setWorkspace((current) => ({ ...current, area }));
      document.querySelector(".workspace")?.scrollTo({ top: 0 });
    },
    [],
  );

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      const typing =
        target !== null && (target.tagName === "INPUT" || target.isContentEditable === true);

      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setPaletteOpen((open) => !open);
        return;
      }
      if (event.key === "Escape") {
        setPaletteOpen(false);
        return;
      }
      if (typing || event.ctrlKey || event.metaKey || event.altKey) return;

      const areaIndex = Number.parseInt(event.key, 10) - 1;
      if (Number.isInteger(areaIndex) && areaIndex >= 0 && areaIndex < AREA_IDS.length) {
        event.preventDefault();
        goTo(AREA_IDS[areaIndex]!);
        return;
      }
      if (event.code === "Space") {
        event.preventDefault();
        session.toggleRun();
        return;
      }
      if (event.key === "ArrowRight") {
        event.preventDefault();
        void session.step();
        return;
      }
      if (event.key.toLowerCase() === "i") {
        event.preventDefault();
        setWorkspace((current) => ({ ...current, dockOpen: !current.dockOpen }));
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [goTo]);

  const areaProps = useMemo<AreaProps>(
    () => ({ workspace, update, goTo }),
    [goTo, update, workspace],
  );

  const area = AREAS[workspace.area];
  const AreaView = area.view;
  const AreaDock = area.dock;
  const attached = connection === "connected" || hasSummary;

  return (
    <>
      <div className="terra" aria-hidden="true" />
      <TerraIncognita />

      <div className="shell" data-rail={workspace.railCollapsed ? "collapsed" : "expanded"}>
        <Meridian
          workspace={workspace}
          update={update}
          onOpenPalette={() => setPaletteOpen(true)}
        />
        <Rail workspace={workspace} goTo={goTo} update={update} />

        <main className="workspace">
          <div className="workspace__inner">
            {error !== undefined && attached && (
              <Notice tone="alarm">
                <span>
                  <b>{copy.connection.errorTitle}.</b> {error}
                </span>
              </Notice>
            )}
            {attached ? <AreaView {...areaProps} /> : <Unattached />}
          </div>
        </main>

        <aside className="dock" hidden={!workspace.dockOpen} aria-label={copy.meridian.inspector}>
          <div className="dock__head">
            <span className="eyebrow">{copy.areas[workspace.area].name}</span>
            <button
              type="button"
              className="btn btn--ghost btn--icon"
              onClick={() => update({ dockOpen: false })}
              aria-label={copy.common.close}
            >
              ×
            </button>
          </div>
          <div className="dock__body">{attached ? <AreaDock {...areaProps} /> : null}</div>
        </aside>

        <Marginalia />
      </div>

      <CommandPalette open={paletteOpen} onClose={() => setPaletteOpen(false)} goTo={goTo} />
    </>
  );
}
